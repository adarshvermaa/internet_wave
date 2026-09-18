/// MAC OUI (Organizationally Unique Identifier) vendor lookup.
/// Maps the first 3 octets of a MAC address to a vendor name and device icon.

#[derive(Debug, Clone)]
pub struct VendorInfo {
    pub name: &'static str,
    #[allow(dead_code)]
    pub icon: &'static str,
}

/// Look up the vendor and device icon from a MAC address string.
/// MAC should be in format "XX:XX:XX:XX:XX:XX" or "XX-XX-XX-XX-XX-XX".
pub fn lookup_vendor(mac: &str) -> VendorInfo {
    let normalized: String = mac.to_uppercase().replace('-', ":");
    if normalized.len() < 8 {
        return VendorInfo { name: "Unknown", icon: "❓" };
    }
    let prefix = &normalized[..8];

    let (name, icon) = match prefix {
        // ── Apple (iPhone, iPad, MacBook, Apple TV) ──
        "3C:15:C2" | "AC:DE:48" | "F0:DB:E2" | "A8:5C:2C" |
        "00:1C:B3" | "00:1E:C2" | "00:25:BC" | "00:26:BB" |
        "28:CF:E9" | "34:C0:59" | "44:D8:84" | "50:32:75" |
        "5C:59:48" | "64:B0:A6" | "70:56:81" | "78:7E:61" |
        "80:E6:50" | "88:66:A5" | "98:01:A7" | "A4:B1:97" |
        "B0:19:C6" | "BC:54:36" | "C8:69:CD" | "D0:C5:F3" |
        "DC:A4:CA" | "E0:B5:2D" | "F0:18:98" => ("Apple", "🍎"),

        // ── Samsung (Galaxy phones, TVs) ──
        "00:16:32" | "00:17:D5" | "00:21:19" | "00:24:54" |
        "08:D4:6A" | "10:D5:42" | "14:49:E0" | "18:3A:2D" |
        "24:4B:03" | "30:CD:A7" | "38:01:95" | "4C:3C:16" |
        "50:01:D9" | "64:77:91" | "78:47:1D" | "8C:77:12" |
        "94:35:0A" | "A0:82:1F" | "BC:D1:1F" | "C4:73:1E" |
        "D0:22:BE" | "E4:7C:F9" | "FC:A1:3E" => ("Samsung", "📱"),

        // ── Xiaomi / Redmi ──
        "00:9E:C8" | "28:6C:07" | "38:A4:ED" | "50:64:2B" |
        "64:09:80" | "74:23:44" | "78:11:DC" | "84:F3:EB" |
        "98:FA:E3" | "AC:C1:EE" | "C4:0B:CB" | "D4:61:DA" |
        "F0:B4:29" | "FC:64:BA" => ("Xiaomi", "📱"),

        // ── Huawei / Honor ──
        "00:E0:FC" | "04:F9:38" | "0C:37:DC" | "10:47:80" |
        "20:08:ED" | "28:31:52" | "48:46:FB" | "54:A5:1B" |
        "70:72:3C" | "80:B6:86" | "88:53:D4" | "AC:E8:7B" |
        "CC:A2:23" | "E8:08:8B" | "FC:48:EF" => ("Huawei", "📱"),

        // ── OnePlus ──
        "94:65:2D" | "C0:EE:FB" | "64:A2:F9" => ("OnePlus", "📱"),

        // ── Google (Pixel, Chromecast, Nest) ──
        "08:9E:08" | "18:D6:C7" | "30:FD:38" | "3C:5A:B4" |
        "54:60:09" | "6C:AD:F8" | "A4:77:33" | "F4:F5:D8" => ("Google", "📱"),

        // ── Intel (WiFi adapters in laptops/desktops) ──
        "00:1B:21" | "00:1C:BF" | "00:1D:E0" | "00:1E:64" |
        "00:1F:3B" | "00:22:FA" | "3C:97:0E" | "4C:34:88" |
        "5C:51:4F" | "68:05:CA" | "8C:8D:28" | "A4:34:D9" |
        "DC:71:96" | "F8:63:3F" => ("Intel", "💻"),

        // ── Realtek (USB WiFi adapters, embedded) ──
        "00:E0:4C" | "48:5D:60" | "52:54:00" | "80:26:89" => ("Realtek", "💻"),

        // ── Qualcomm / Atheros ──
        "00:03:7F" | "00:0E:6D" | "00:13:74" | "00:1B:EB" |
        "00:24:D7" | "1C:B7:2C" | "5C:F3:70" | "9C:B7:0D" => ("Qualcomm", "💻"),

        // ── Raspberry Pi ──
        "28:CD:C1" | "B8:27:EB" | "D8:3A:DD" | "DC:A6:32" |
        "E4:5F:01" => ("Raspberry Pi", "🖥️"),

        // ── Microsoft (Xbox, Surface) ──
        "00:50:F2" | "28:18:78" | "7C:1E:52" | "C8:3F:26" |
        "00:15:5D" | "00:1D:D8" => ("Microsoft", "🖥️"),

        // ── Sony (PlayStation) ──
        "00:04:1F" | "00:13:15" | "00:1D:0D" | "28:0D:FC" |
        "70:9E:29" | "A8:E3:EE" | "FC:0F:E6" => ("Sony", "🎮"),

        // ── Amazon (Echo, Fire, Ring) ──
        "00:FC:8B" | "10:CE:A9" | "18:74:2E" | "34:D2:70" |
        "44:65:0D" | "50:DC:E7" | "68:54:FD" | "74:C2:46" |
        "84:D6:D0" | "AC:63:BE" | "F0:27:2D" | "FC:65:DE" => ("Amazon", "📺"),

        // ── LG (TVs, phones) ──
        "00:1E:75" | "10:68:3F" | "20:3D:BD" | "40:B8:9A" |
        "58:A2:B5" | "78:5D:C8" | "A8:16:B2" | "CC:FA:00" => ("LG", "📺"),

        // ── TP-Link (routers, access points) ──
        "14:CF:92" | "30:B5:C2" | "50:C7:BF" | "54:C8:0F" |
        "60:E3:27" | "70:4F:57" | "78:44:76" | "98:DA:C4" |
        "B0:4E:26" | "C0:25:E9" | "D4:6E:0E" | "EC:08:6B" |
        "F4:F2:6D" | "F8:1A:67" => ("TP-Link", "🌐"),

        // ── D-Link ──
        "00:05:5D" | "00:0D:88" | "00:17:9A" | "00:1B:11" |
        "00:1E:58" | "14:D6:4D" | "28:10:7B" | "78:54:2E" |
        "B8:A3:86" | "C8:BE:19" | "F0:7D:68" => ("D-Link", "🌐"),

        // ── Netgear ──
        "00:09:5B" | "00:0F:B5" | "00:14:6C" | "00:1B:2F" |
        "00:1E:2A" | "00:22:3F" | "08:BD:43" | "20:0C:C8" |
        "44:94:FC" | "6C:B0:CE" | "84:1B:5E" | "A0:04:60" |
        "C4:04:15" | "DC:EF:09" => ("Netgear", "🌐"),

        // ── ASUS (routers, laptops) ──
        "00:0C:6E" | "00:11:2F" | "00:1A:92" | "00:1D:60" |
        "00:22:15" | "04:D4:C4" | "08:60:6E" | "10:BF:48" |
        "2C:4D:54" | "40:16:7E" | "54:04:A6" | "AC:22:0B" |
        "D4:5D:64" | "F4:6D:04" => ("ASUS", "🌐"),

        // ── Linksys ──
        "00:06:25" | "00:0C:41" | "00:14:BF" | "00:18:39" |
        "00:1A:70" | "00:1D:7E" | "00:22:6B" | "20:AA:4B" |
        "58:6D:8F" | "C0:56:27" => ("Linksys", "🌐"),

        // ── Cisco ──
        "00:00:0C" | "00:01:42" | "00:01:64" | "00:04:27" |
        "00:0A:41" | "00:0D:BC" | "00:12:00" | "00:14:69" |
        "00:16:46" | "00:18:73" | "00:1A:2F" | "00:1C:57" => ("Cisco", "🌐"),

        // ── Espressif (ESP32, ESP8266 IoT) ──
        "24:0A:C4" | "24:6F:28" | "30:AE:A4" | "3C:61:05" |
        "3C:71:BF" | "4C:11:AE" | "5C:CF:7F" | "60:01:94" |
        "68:C6:3A" | "84:CC:A8" | "A0:20:A6" |
        "A4:7B:9D" | "A4:CF:12" | "AC:67:B2" | "B4:E6:2D" |
        "BC:DD:C2" | "C4:4F:33" | "CC:50:E3" | "EC:FA:BC" => ("ESP/IoT", "💡"),

        _ => ("Unknown", "❓"),
    };

    VendorInfo { name, icon }
}

/// Guess a more specific device type icon based on vendor.
/// Routers get 🌐, phones get 📱, etc.
pub fn device_type_icon(vendor: &str, is_gateway: bool) -> &'static str {
    if is_gateway {
        return "🌐";
    }
    match vendor {
        "Apple" | "Samsung" | "Xiaomi" | "Huawei" | "OnePlus" | "Google" => "📱",
        "Intel" | "Realtek" | "Qualcomm" | "Microsoft" => "💻",
        "Raspberry Pi" => "🖥️",
        "Sony" => "🎮",
        "Amazon" | "LG" => "📺",
        "ESP/IoT" => "💡",
        "TP-Link" | "D-Link" | "Netgear" | "ASUS" | "Linksys" | "Cisco" => "🌐",
        _ => "❓",
    }
}
