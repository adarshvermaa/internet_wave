# 📡 WiFi Wave v3

**Animated Network Visualizer, Proximity Radar & Real-Time Traffic Inspector for your Terminal.**

WiFi Wave scans your local network, estimates real-time distance to every connected device, pulses animated sonar waves from your router, and **inspects live traffic to show what websites and streaming services each person is using in real time.**

```
  📡 WiFi Wave v3 │ Interface: enp2s0 (192.168.31.190) │ Mode: Ethernet/LAN │ 🚨 1 NEW DEVICE(S)! 
  ╭──────────────────── Network Radar & Live Activities ──────────────────╮
  │                                                                       │
  │               📱 iPhone (0.8ms ~1.5m) [🎬 YouTube] ✓                  │
  │                               ·                                       │
  │                  ·            ·                                       │
  │              ~ ~ ~ ~ ~ ~      ·                                       │
  │           ~    ·    ·    ~    ·                                       │
  │         ~   ·    🌐    ·   ~  ·                                       │
  │           ~   192.168.31.1 ~                                          │
  │              ~ ~ ~ ~ ~ ~                                              │
  │                   ·                     💻 Laptop (4.2ms ~6.5m) [💬 WhatsApp] ⚠️ NEW │
  │                   ·                                                   │
  │            🖥️ Desktop (1.2ms ~2.2m) [💼 GitHub] ✓                       │
  │                                                                       │
  ╰───────────────────────────────────────────────────────────────────────╯
  ╭────────────────── 🌐 Live Network & Website Traffic Feed ─────────────╮
  │   192.168.31.45  → rr1---sn-xxx.googlevideo.com   (🎬 YouTube) [DNS]   │
  │   192.168.31.102 → web.whatsapp.com               (💬 WhatsApp) [HTTPS]│
  │   192.168.31.88  → Chromecast: Living Room TV     (🎬 Media Cast)      │
  ╰───────────────────────────────────────────────────────────────────────╯
   ✓ 3 devices │ GW: 192.168.31.1 │ ▼ 2.4 MB/s  ▲ 142 KB/s │ ↑↓ Select │ s Save Known │ r Rescan │ q Quit 
```

---

## 🚀 What WiFi Wave Does

### 1. 🌐 Real-Time Website & Traffic Inspection
- **Passive DNS Sniffer (UDP 53)**: Captures DNS queries in real time to see which websites each device is resolving.
- **TLS ClientHello SNI Parser (TCP 443)**: Extracts the destination domain name from HTTPS handshakes before encryption.
- **Service Classifier**: Automatically identifies platforms:
  - 🎬 **Video Streaming**: YouTube, Netflix, Disney+, Prime Video, Twitch, Vimeo
  - 💬 **Social & Chat**: WhatsApp, Instagram, Telegram, TikTok, Facebook, X (Twitter), Reddit, Discord
  - 🎵 **Music Streaming**: Spotify, Apple Music, SoundCloud
  - 🎮 **Gaming**: Steam, Epic Games, PlayStation Network, Xbox Live, Roblox, Riot Games (Valorant)
  - 💼 **Work & Productivity**: Zoom, MS Teams, Slack, Google Workspace, GitHub
- **Active Service Badges**: Badges like `[🎬 YouTube]` and `[💬 WhatsApp]` appear directly next to each device on the radar map!

### 2. 📺 Multicast Media Streaming Discovery (Chromecast, Spotify, AirPlay)
- Listens to **mDNS (UDP 5353)** and **SSDP (UDP 1900)** broadcasts.
- Discovers when anyone on the network casts YouTube/Netflix to a smart TV or streams audio via Spotify Connect or Apple AirPlay.

### 3. 📏 Distance & Proximity Estimation
- Measures ping latency (RTT) to every device in parallel.
- Estimates distance in meters (`~1.5m`, `~3.2m`, etc.).
- **Dynamic Radial Radar**: Devices physically closer to the router orbit closer to the center; distant devices orbit further out!

### 4. 🚨 New Device Alerts & Whitelist
- Compares connected devices against your saved whitelist.
- Highlights untrusted/new devices with a flashing red `⚠️ NEW` badge and red connection lines.
- Press **`s`** to save all current devices to `~/.config/wifi-wave/known_devices.txt`.

### 5. 📊 Real-Time Bandwidth & Signal Meter
- Live download (`▼`) and upload (`▲`) speed metrics from interface statistics.
- WiFi signal strength bars (`▂▄▆█`), dBm, and link quality.

---

## 🎮 Controls

| Key | Action |
|---|---|
| `↑` / `k` | Select previous device |
| `↓` / `j` | Select next device |
| `Tab` | Cycle selection |
| `s` | **Save / Whitelist** current devices as known |
| `r` | Force immediate rescan & distance measurement |
| `q` / `Esc` | Quit |

---

## 🛠️ Build & Run

```bash
# Build optimized release binary
cargo build --release

# Run (requires sudo for raw packet capture and ARP sweeps)
sudo ./target/release/wifi-wave
```

---

## 🔒 Privacy & Technical Reality

- **What CAN be seen**: Domain names (e.g. `youtube.com`, `web.whatsapp.com`), streaming sessions (Chromecast, Spotify), and connection speeds.
- **What CANNOT be seen (Encrypted)**: Individual video URLs, private messages, passwords, and personal search queries remain 100% end-to-end encrypted by modern HTTPS/TLS.

## License

MIT