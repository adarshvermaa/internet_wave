/// Application state module.
/// Holds the list of discovered devices, animation state, UI selection,
/// bandwidth tracker, alert state, and real-time network activity monitor.

use std::collections::{HashMap, VecDeque};
use std::net::Ipv4Addr;
use std::time::Instant;

use crate::alerts::{AlertState, KnownDevices};
use crate::animation::{ConnectionAnim, WaveState};
use crate::classifier::{classify_domain, ServiceCategory};
use crate::discovery::DiscoveredMedia;
use crate::scanner::{self, Device, InterfaceInfo, WifiSignal};
use crate::sniffer::PacketActivity;

/// Possible application actions produced by input handling.
pub enum AppAction {
    Quit,
    ForceRescan,
    SaveKnownDevices,
    None,
}

/// Scan status indicator.
#[derive(Debug, Clone, PartialEq)]
pub enum ScanStatus {
    Scanning,
    Done { device_count: usize },
}

// ─── Network activity items ──────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FeedItem {
    pub ip: Ipv4Addr,
    pub domain: String,
    pub service_name: String,
    pub category: ServiceCategory,
    pub protocol: &'static str,
}

#[derive(Debug, Clone, Default)]
pub struct DeviceActivity {
    pub recent_domains: VecDeque<(String, ServiceCategory)>,
    pub active_services: Vec<String>,
}

// ─── Bandwidth tracker ───────────────────────────────────────────────

/// Tracks interface bandwidth over time for display.
pub struct BandwidthTracker {
    pub history_rx: VecDeque<u64>,
    pub history_tx: VecDeque<u64>,
    last_rx_bytes: u64,
    last_tx_bytes: u64,
    last_sample_time: Option<Instant>,
    pub current_rx: f64,
    pub current_tx: f64,
    tick_counter: u64,
}

impl BandwidthTracker {
    pub fn new() -> Self {
        Self {
            history_rx: VecDeque::with_capacity(61),
            history_tx: VecDeque::with_capacity(61),
            last_rx_bytes: 0,
            last_tx_bytes: 0,
            last_sample_time: None,
            current_rx: 0.0,
            current_tx: 0.0,
            tick_counter: 0,
        }
    }

    pub fn tick(&mut self, iface_name: &str) {
        self.tick_counter += 1;
        if self.tick_counter % 30 != 0 {
            return;
        }

        if let Some((rx, tx)) = scanner::read_interface_bytes(iface_name) {
            let now = Instant::now();
            if let Some(last_time) = self.last_sample_time {
                let elapsed = now.duration_since(last_time).as_secs_f64();
                if elapsed > 0.01 {
                    self.current_rx = (rx.saturating_sub(self.last_rx_bytes)) as f64 / elapsed;
                    self.current_tx = (tx.saturating_sub(self.last_tx_bytes)) as f64 / elapsed;

                    self.history_rx.push_back(self.current_rx as u64);
                    self.history_tx.push_back(self.current_tx as u64);

                    if self.history_rx.len() > 60 {
                        self.history_rx.pop_front();
                        self.history_tx.pop_front();
                    }
                }
            }
            self.last_rx_bytes = rx;
            self.last_tx_bytes = tx;
            self.last_sample_time = Some(now);
        }
    }
}

pub fn format_bandwidth(bps: f64) -> String {
    if bps < 1024.0 {
        format!("{:.0} B/s", bps)
    } else if bps < 1024.0 * 1024.0 {
        format!("{:.1} KB/s", bps / 1024.0)
    } else if bps < 1024.0 * 1024.0 * 1024.0 {
        format!("{:.1} MB/s", bps / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB/s", bps / (1024.0 * 1024.0 * 1024.0))
    }
}

// ─── Main application state ─────────────────────────────────────────

pub struct App {
    pub devices_map: HashMap<Ipv4Addr, Device>,
    pub devices: Vec<Device>,
    pub waves: WaveState,
    pub conn_anim: ConnectionAnim,
    pub selected: Option<usize>,
    pub scan_status: ScanStatus,
    pub interface: InterfaceInfo,
    pub running: bool,
    pub tick_count: u64,
    pub alerts: AlertState,
    pub known_devices: KnownDevices,
    pub bandwidth: BandwidthTracker,
    pub wifi_signal: Option<WifiSignal>,

    // Real-time network activity
    pub activity_feed: VecDeque<FeedItem>,
    pub device_activities: HashMap<Ipv4Addr, DeviceActivity>,
}

impl App {
    pub fn new(interface: InterfaceInfo) -> Self {
        let known_devices = KnownDevices::load();
        Self {
            devices_map: HashMap::new(),
            devices: Vec::new(),
            waves: WaveState::new(),
            conn_anim: ConnectionAnim::new(),
            selected: None,
            scan_status: ScanStatus::Scanning,
            interface,
            running: true,
            tick_count: 0,
            alerts: AlertState::new(),
            known_devices,
            bandwidth: BandwidthTracker::new(),
            wifi_signal: None,
            activity_feed: VecDeque::with_capacity(32),
            device_activities: HashMap::new(),
        }
    }

    pub fn tick(&mut self) {
        self.tick_count += 1;
        self.waves.tick();
        self.conn_anim.tick();
        self.alerts.tick();
        self.bandwidth.tick(&self.interface.name);

        if self.tick_count % 150 == 0 {
            self.wifi_signal = scanner::get_wifi_signal();
        }
    }

    pub fn record_packet_activity(&mut self, act: PacketActivity) {
        let service = classify_domain(&act.domain);
        let item = FeedItem {
            ip: act.ip,
            domain: act.domain.clone(),
            service_name: service.service_name.clone(),
            category: service.category,
            protocol: act.protocol,
        };

        self.activity_feed.push_front(item);
        if self.activity_feed.len() > 25 {
            self.activity_feed.pop_back();
        }

        let dev_act = self.device_activities.entry(act.ip).or_default();
        if !dev_act.active_services.contains(&service.service_name) {
            dev_act.active_services.push(service.service_name);
            if dev_act.active_services.len() > 3 {
                dev_act.active_services.remove(0);
            }
        }

        dev_act.recent_domains.push_front((act.domain, service.category));
        if dev_act.recent_domains.len() > 15 {
            dev_act.recent_domains.pop_back();
        }
    }

    pub fn record_media_activity(&mut self, media: DiscoveredMedia) {
        let service_display = if let Some(ref detail) = media.detail {
            format!("{}: {}", media.service, detail)
        } else {
            media.service.clone()
        };

        let item = FeedItem {
            ip: media.ip,
            domain: service_display,
            service_name: media.service.clone(),
            category: ServiceCategory::VideoStreaming,
            protocol: "mDNS/SSDP",
        };

        self.activity_feed.push_front(item);
        if self.activity_feed.len() > 25 {
            self.activity_feed.pop_back();
        }

        let dev_act = self.device_activities.entry(media.ip).or_default();
        if !dev_act.active_services.contains(&media.service) {
            dev_act.active_services.push(media.service);
            if dev_act.active_services.len() > 3 {
                dev_act.active_services.remove(0);
            }
        }
    }

    pub fn update_devices(&mut self, new_devices: Vec<Device>) {
        for device in &new_devices {
            self.devices_map
                .entry(device.ip)
                .and_modify(|existing| {
                    existing.last_seen = device.last_seen;
                    existing.hostname.clone_from(&device.hostname);
                    existing.latency_ms = device.latency_ms;
                    existing.estimated_distance_m = device.estimated_distance_m;
                })
                .or_insert_with(|| device.clone());
        }

        let mut sorted: Vec<Device> = self.devices_map.values().cloned().collect();
        sorted.sort_by(|a, b| {
            b.is_gateway
                .cmp(&a.is_gateway)
                .then_with(|| {
                    let la = a.latency_ms.unwrap_or(999.0);
                    let lb = b.latency_ms.unwrap_or(999.0);
                    la.partial_cmp(&lb).unwrap_or(std::cmp::Ordering::Equal)
                })
        });
        self.devices = sorted;

        self.alerts.update(&self.devices, &self.known_devices);

        if let Some(sel) = self.selected {
            if sel >= self.devices.len() {
                self.selected = if self.devices.is_empty() {
                    None
                } else {
                    Some(self.devices.len() - 1)
                };
            }
        }

        self.scan_status = ScanStatus::Done {
            device_count: self.devices.len(),
        };
    }

    pub fn save_known_devices(&mut self) -> Result<(), String> {
        self.known_devices
            .save_all(&self.devices)
            .map_err(|e| format!("Failed to save: {e}"))?;
        self.alerts.update(&self.devices, &self.known_devices);
        Ok(())
    }

    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> AppAction {
        use crossterm::event::KeyCode;

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => {
                self.running = false;
                AppAction::Quit
            }
            KeyCode::Char('r') => {
                self.scan_status = ScanStatus::Scanning;
                AppAction::ForceRescan
            }
            KeyCode::Char('s') => AppAction::SaveKnownDevices,
            KeyCode::Down | KeyCode::Char('j') => {
                self.select_next();
                AppAction::None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.select_prev();
                AppAction::None
            }
            KeyCode::Tab => {
                self.select_next();
                AppAction::None
            }
            _ => AppAction::None,
        }
    }

    fn select_next(&mut self) {
        if self.devices.is_empty() {
            return;
        }
        self.selected = Some(match self.selected {
            Some(i) => (i + 1) % self.devices.len(),
            None => 0,
        });
    }

    fn select_prev(&mut self) {
        if self.devices.is_empty() {
            return;
        }
        self.selected = Some(match self.selected {
            Some(0) => self.devices.len() - 1,
            Some(i) => i - 1,
            None => self.devices.len() - 1,
        });
    }
}
