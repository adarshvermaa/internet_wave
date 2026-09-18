/// New device alert system.
/// Tracks known devices via a persistent whitelist file and provides
/// flashing alert state for newly discovered (unknown) devices.

use std::collections::HashSet;
use std::path::PathBuf;

use crate::scanner::Device;

// ─── Known devices whitelist ─────────────────────────────────────────

/// Persistent whitelist of known MAC addresses.
pub struct KnownDevices {
    known_macs: HashSet<String>,
    config_path: PathBuf,
}

impl KnownDevices {
    /// Load known devices from ~/.config/wifi-wave/known_devices.txt
    pub fn load() -> Self {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let config_dir = PathBuf::from(home).join(".config").join("wifi-wave");
        let config_path = config_dir.join("known_devices.txt");

        let known_macs = if config_path.exists() {
            std::fs::read_to_string(&config_path)
                .unwrap_or_default()
                .lines()
                .filter(|l| !l.trim().is_empty() && !l.starts_with('#'))
                .map(|l| l.trim().to_uppercase())
                .collect()
        } else {
            HashSet::new()
        };

        KnownDevices {
            known_macs,
            config_path,
        }
    }

    /// Check if a MAC address is in the known whitelist.
    pub fn is_known(&self, mac: &str) -> bool {
        self.known_macs.contains(&mac.to_uppercase())
    }

    /// Save all current devices to the whitelist file.
    pub fn save_all(&mut self, devices: &[Device]) -> std::io::Result<()> {
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut content = String::from("# WiFi Wave — Known Devices\n");
        content.push_str("# One MAC address per line. Devices not in this list trigger alerts.\n\n");

        for device in devices {
            let hostname = device
                .hostname
                .as_deref()
                .unwrap_or("unknown");
            content.push_str(&format!(
                "{}  # {} ({}) {}\n",
                device.mac, device.ip, device.vendor, hostname
            ));
            self.known_macs.insert(device.mac.clone());
        }

        std::fs::write(&self.config_path, &content)?;
        Ok(())
    }

    /// Number of known devices.
    #[allow(dead_code)]
    pub fn count(&self) -> usize {
        self.known_macs.len()
    }
}

// ─── Alert animation state ──────────────────────────────────────────

/// Manages flashing animation for newly discovered devices.
pub struct AlertState {
    /// MACs that are not in the known whitelist.
    new_macs: HashSet<String>,
    /// Whether the flash is currently visible (toggles for blink effect).
    pub flash_visible: bool,
    /// Internal counter for flash timing.
    flash_counter: u64,
    /// Total number of new (unknown) devices.
    pub new_count: usize,
}

impl AlertState {
    pub fn new() -> Self {
        Self {
            new_macs: HashSet::new(),
            flash_visible: true,
            flash_counter: 0,
            new_count: 0,
        }
    }

    /// Advance the flash animation (call every tick ≈ 33ms).
    pub fn tick(&mut self) {
        self.flash_counter += 1;
        // Toggle visibility every ~15 ticks (≈ 500ms at 30fps)
        if self.flash_counter % 15 == 0 {
            self.flash_visible = !self.flash_visible;
        }
    }

    /// Update the alert state with the current device list.
    pub fn update(&mut self, devices: &[Device], known: &KnownDevices) {
        self.new_macs.clear();
        for device in devices {
            if !known.is_known(&device.mac) {
                self.new_macs.insert(device.mac.clone());
            }
        }
        self.new_count = self.new_macs.len();
    }

    /// Check if a specific device is new (unknown).
    pub fn is_new(&self, mac: &str) -> bool {
        self.new_macs.contains(mac)
    }

    /// Whether there are any new devices.
    pub fn has_alerts(&self) -> bool {
        self.new_count > 0
    }
}
