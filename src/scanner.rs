/// Network scanner module.
/// Discovers devices on the local WiFi network by:
/// 1. Detecting the active network interface via `ip route`
/// 2. Running a parallel ping sweep to populate the ARP cache
/// 3. Reading /proc/net/arp for discovered MAC addresses
/// 4. Measuring ping latency to each device for distance estimation
/// 5. Resolving hostnames via reverse DNS
/// 6. Reading WiFi signal strength from /proc/net/wireless

use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::str::FromStr;
use std::time::Instant;

use ipnetwork::Ipv4Network;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

use crate::vendor;

// ─── Data types ──────────────────────────────────────────────────────

/// A discovered network device.
#[derive(Debug, Clone)]
pub struct Device {
    pub ip: Ipv4Addr,
    pub mac: String,
    pub hostname: Option<String>,
    pub vendor: String,
    pub icon: String,
    pub is_gateway: bool,
    pub first_seen: Instant,
    pub last_seen: Instant,
    /// Ping round-trip time in milliseconds.
    pub latency_ms: Option<f64>,
    /// Estimated distance in meters (from latency or RSSI).
    pub estimated_distance_m: Option<f64>,
}

/// Information about the active network interface.
#[derive(Debug, Clone)]
pub struct InterfaceInfo {
    pub name: String,
    pub ip: Ipv4Addr,
    #[allow(dead_code)]
    pub mac: String,
    pub network: Ipv4Network,
    pub gateway: Option<Ipv4Addr>,
}

/// WiFi signal info for our own connection.
#[derive(Debug, Clone)]
pub struct WifiSignal {
    #[allow(dead_code)]
    pub interface: String,
    pub rssi_dbm: i32,
    pub distance_m: f64,
    pub quality: SignalQuality,
}

#[derive(Debug, Clone, Copy)]
pub enum SignalQuality {
    Excellent, // > -50 dBm
    Good,      // -50 to -60
    Fair,      // -60 to -70
    Weak,      // -70 to -80
    VeryWeak,  // < -80
}

impl SignalQuality {
    pub fn from_dbm(dbm: i32) -> Self {
        match dbm {
            d if d > -50 => Self::Excellent,
            d if d > -60 => Self::Good,
            d if d > -70 => Self::Fair,
            d if d > -80 => Self::Weak,
            _ => Self::VeryWeak,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Excellent => "Excellent",
            Self::Good => "Good",
            Self::Fair => "Fair",
            Self::Weak => "Weak",
            Self::VeryWeak => "Very Weak",
        }
    }

    pub fn bars(&self) -> &'static str {
        match self {
            Self::Excellent => "▂▄▆█",
            Self::Good => "▂▄▆░",
            Self::Fair => "▂▄░░",
            Self::Weak => "▂░░░",
            Self::VeryWeak => "░░░░",
        }
    }
}

// ─── Interface detection ─────────────────────────────────────────────

/// Auto-detect the active network interface by parsing `ip route` and `ip addr`.
pub fn detect_interface() -> Result<InterfaceInfo, String> {
    let output = std::process::Command::new("ip")
        .args(["route", "show", "default"])
        .output()
        .map_err(|e| format!("Failed to run 'ip route': {e}"))?;

    let route_str = String::from_utf8_lossy(&output.stdout);
    let parts: Vec<&str> = route_str.split_whitespace().collect();

    let gateway = parts
        .iter()
        .position(|&p| p == "via")
        .and_then(|i| parts.get(i + 1))
        .and_then(|s| Ipv4Addr::from_str(s).ok());

    let iface_name = parts
        .iter()
        .position(|&p| p == "dev")
        .and_then(|i| parts.get(i + 1))
        .ok_or("Could not determine interface from 'ip route show default'. Are you connected to WiFi?")?
        .to_string();

    let output = std::process::Command::new("ip")
        .args(["-4", "addr", "show", &iface_name])
        .output()
        .map_err(|e| format!("Failed to run 'ip addr': {e}"))?;

    let addr_str = String::from_utf8_lossy(&output.stdout);

    let ip_cidr = addr_str
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.starts_with("inet ") {
                line.split_whitespace().nth(1)
            } else {
                None
            }
        })
        .next()
        .ok_or(format!("No IPv4 address found on interface '{iface_name}'"))?;

    let network = Ipv4Network::from_str(ip_cidr)
        .map_err(|e| format!("Could not parse CIDR '{ip_cidr}': {e}"))?;
    let ip = network.ip();

    let mac_path = format!("/sys/class/net/{iface_name}/address");
    let mac = std::fs::read_to_string(&mac_path)
        .map_err(|e| format!("Could not read MAC from '{mac_path}': {e}"))?
        .trim()
        .to_uppercase();

    Ok(InterfaceInfo {
        name: iface_name,
        ip,
        mac,
        network,
        gateway,
    })
}

// ─── Network scanning ───────────────────────────────────────────────

/// Perform a full network scan: ping sweep + ARP table parse.
pub async fn scan_network(info: &InterfaceInfo) -> Vec<(Ipv4Addr, String)> {
    let ips: Vec<Ipv4Addr> = info
        .network
        .iter()
        .filter(|&ip| ip != info.ip && ip != info.network.network() && ip != info.network.broadcast())
        .collect();

    let batch_size = 64;
    for batch in ips.chunks(batch_size) {
        let handles: Vec<_> = batch
            .iter()
            .map(|&ip| {
                tokio::spawn(async move {
                    let _ = timeout(
                        Duration::from_secs(1),
                        Command::new("ping")
                            .args(["-c", "1", "-W", "1", &ip.to_string()])
                            .stdout(std::process::Stdio::null())
                            .stderr(std::process::Stdio::null())
                            .output(),
                    )
                    .await;
                })
            })
            .collect();

        for h in handles {
            let _ = h.await;
        }
    }

    tokio::time::sleep(Duration::from_millis(200)).await;
    parse_arp_table(&info.name)
}

/// Read and parse /proc/net/arp for the given interface.
fn parse_arp_table(iface_name: &str) -> Vec<(Ipv4Addr, String)> {
    let content = match std::fs::read_to_string("/proc/net/arp") {
        Ok(c) => c,
        Err(_) => return vec![],
    };

    let mut results = vec![];
    for line in content.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 6 {
            continue;
        }
        let ip_str = parts[0];
        let flags = parts[2];
        let mac = parts[3];
        let device = parts[5];

        if flags == "0x0" || device != iface_name || mac == "00:00:00:00:00:00" {
            continue;
        }

        if let Ok(ip) = Ipv4Addr::from_str(ip_str) {
            results.push((ip, mac.to_uppercase()));
        }
    }
    results
}

// ─── Latency measurement ─────────────────────────────────────────────

/// Measure ping latency to a single IP address. Returns RTT in milliseconds.
pub async fn measure_latency(ip: Ipv4Addr) -> Option<f64> {
    let output = timeout(
        Duration::from_secs(2),
        Command::new("ping")
            .args(["-c", "1", "-W", "1", &ip.to_string()])
            .output(),
    )
    .await
    .ok()?
    .ok()?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Look for "time=X.XX ms" or "time=X.XX"
    for word in stdout.split_whitespace() {
        if let Some(time_val) = word.strip_prefix("time=") {
            return time_val.parse::<f64>().ok();
        }
    }
    None
}

/// Measure latency to all discovered devices in parallel.
pub async fn measure_all_latencies(ips: &[Ipv4Addr]) -> HashMap<Ipv4Addr, f64> {
    let mut results = HashMap::new();

    let handles: Vec<_> = ips
        .iter()
        .map(|&ip| tokio::spawn(async move { (ip, measure_latency(ip).await) }))
        .collect();

    for h in handles {
        if let Ok((ip, Some(latency))) = h.await {
            results.insert(ip, latency);
        }
    }

    results
}

// ─── Distance estimation ─────────────────────────────────────────────

/// Estimate distance from WiFi RSSI using Log-Distance Path Loss Model.
/// TxPower ≈ -30 dBm (typical at 1 meter), n ≈ 3.0 (indoor).
pub fn estimate_distance_from_rssi(rssi: i32) -> f64 {
    let tx_power = -30.0_f64;
    let n = 3.0_f64;
    let distance = 10.0_f64.powf((tx_power - rssi as f64) / (10.0 * n));
    (distance * 10.0).round() / 10.0
}

/// Estimate rough distance from ping latency (heuristic for LAN).
/// This is NOT physical distance — it's a rough proximity indicator.
pub fn estimate_distance_from_latency(latency_ms: f64) -> f64 {
    // On a home LAN, latency roughly correlates with proximity:
    // < 1ms:  directly connected, same switch → ~1-2m
    // 1-3ms:  nearby, through one switch hop → ~3-5m
    // 3-10ms: further, through router/repeater → ~5-10m
    // >10ms:  far away or congested → ~10-15m
    if latency_ms < 0.5 {
        1.0
    } else if latency_ms < 1.0 {
        1.5 + latency_ms
    } else if latency_ms < 3.0 {
        3.0 + latency_ms
    } else if latency_ms < 10.0 {
        5.0 + latency_ms * 0.5
    } else {
        10.0 + (latency_ms - 10.0).min(5.0)
    }
}

// ─── WiFi signal strength ────────────────────────────────────────────

/// Read WiFi signal strength from any wireless interface.
/// Returns signal info if a wireless interface is connected.
pub fn get_wifi_signal() -> Option<WifiSignal> {
    // Try /proc/net/wireless
    if let Ok(content) = std::fs::read_to_string("/proc/net/wireless") {
        for line in content.lines().skip(2) {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                let iface = parts[0].trim_end_matches(':').to_string();
                if let Ok(level) = parts[3].trim_end_matches('.').parse::<f64>() {
                    let dbm = level as i32;
                    // If positive, it's a relative value; convert to dBm
                    let dbm = if dbm > 0 && dbm < 100 { dbm - 256 } else { dbm };
                    if dbm < 0 {
                        let distance = estimate_distance_from_rssi(dbm);
                        let quality = SignalQuality::from_dbm(dbm);
                        return Some(WifiSignal {
                            interface: iface,
                            rssi_dbm: dbm,
                            distance_m: distance,
                            quality,
                        });
                    }
                }
            }
        }
    }

    // Fallback: try `iw dev <iface> link` for each wireless interface
    if let Ok(entries) = std::fs::read_dir("/sys/class/net") {
        for entry in entries.flatten() {
            let iface = entry.file_name().to_string_lossy().to_string();
            let wireless_path = format!("/sys/class/net/{iface}/wireless");
            if std::path::Path::new(&wireless_path).exists() {
                if let Ok(output) = std::process::Command::new("iw")
                    .args(["dev", &iface, "link"])
                    .output()
                {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    for line in stdout.lines() {
                        let line = line.trim();
                        if let Some(rest) = line.strip_prefix("signal:") {
                            if let Some(dbm) = rest.split_whitespace().next() {
                                if let Ok(dbm) = dbm.parse::<i32>() {
                                    let distance = estimate_distance_from_rssi(dbm);
                                    let quality = SignalQuality::from_dbm(dbm);
                                    return Some(WifiSignal {
                                        interface: iface,
                                        rssi_dbm: dbm,
                                        distance_m: distance,
                                        quality,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    None
}

// ─── Bandwidth reading ───────────────────────────────────────────────

/// Read raw RX/TX byte counters from sysfs for the given interface.
pub fn read_interface_bytes(iface_name: &str) -> Option<(u64, u64)> {
    let rx_path = format!("/sys/class/net/{iface_name}/statistics/rx_bytes");
    let tx_path = format!("/sys/class/net/{iface_name}/statistics/tx_bytes");

    let rx: u64 = std::fs::read_to_string(&rx_path).ok()?.trim().parse().ok()?;
    let tx: u64 = std::fs::read_to_string(&tx_path).ok()?.trim().parse().ok()?;

    Some((rx, tx))
}

// ─── Device enrichment ──────────────────────────────────────────────

/// Enrich raw (IP, MAC) scan results into full Device structs, including latency.
pub fn enrich_devices(
    scan_results: &[(Ipv4Addr, String)],
    gateway: Option<Ipv4Addr>,
    existing: &HashMap<Ipv4Addr, Device>,
    latencies: &HashMap<Ipv4Addr, f64>,
) -> Vec<Device> {
    let now = Instant::now();

    scan_results
        .iter()
        .map(|(ip, mac)| {
            let info = vendor::lookup_vendor(mac);
            let is_gateway = gateway.map_or(false, |gw| gw == *ip);
            let icon = vendor::device_type_icon(info.name, is_gateway).to_string();

            let ip_addr: std::net::IpAddr = (*ip).into();
            let hostname = dns_lookup::lookup_addr(&ip_addr).ok();

            let first_seen = existing.get(ip).map(|d| d.first_seen).unwrap_or(now);

            let latency_ms = latencies.get(ip).copied();
            let estimated_distance_m =
                latency_ms.map(estimate_distance_from_latency);

            Device {
                ip: *ip,
                mac: mac.clone(),
                hostname,
                vendor: info.name.to_string(),
                icon,
                is_gateway,
                first_seen,
                last_seen: now,
                latency_ms,
                estimated_distance_m,
            }
        })
        .collect()
}

/// Full scan pipeline: ping sweep → ARP parse → latency measurement → enrichment.
pub async fn full_scan(info: &InterfaceInfo) -> Vec<Device> {
    let raw_results = scan_network(info).await;
    let ips: Vec<Ipv4Addr> = raw_results.iter().map(|(ip, _)| *ip).collect();
    let latencies = measure_all_latencies(&ips).await;
    enrich_devices(&raw_results, info.gateway, &HashMap::new(), &latencies)
}
