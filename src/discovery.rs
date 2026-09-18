/// Multicast service discovery module.
/// Listens to mDNS (UDP 5353) and SSDP (UDP 1900) broadcasts across the local WiFi/LAN
/// to discover active Google Cast / Chromecast, Spotify Connect, AirPlay, and smart TV sessions.

use std::net::{Ipv4Addr, SocketAddrV4};
use tokio::net::UdpSocket;
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct DiscoveredMedia {
    pub ip: Ipv4Addr,
    pub service: String,
    pub detail: Option<String>,
}

/// Start background listeners for mDNS and SSDP.
pub async fn start_discovery(tx: Sender<DiscoveredMedia>) {
    // 1. mDNS Listener (UDP 5353 -> 224.0.0.251)
    let mdns_tx = tx.clone();
    tokio::spawn(async move {
        let bind_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 5353);
        let multicast_ip = Ipv4Addr::new(224, 0, 0, 251);

        if let Ok(socket) = UdpSocket::bind(bind_addr).await {
            let _ = socket.join_multicast_v4(multicast_ip, Ipv4Addr::UNSPECIFIED);

            let mut buf = [0u8; 4096];
            while let Ok((len, from)) = socket.recv_from(&mut buf).await {
                if let std::net::SocketAddr::V4(v4) = from {
                    let packet = &buf[..len];
                    if let Some(media) = parse_mdns_packet(*v4.ip(), packet) {
                        if mdns_tx.send(media).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    });

    // 2. SSDP Listener (UDP 1900 -> 239.255.255.250)
    let ssdp_tx = tx;
    tokio::spawn(async move {
        let bind_addr = SocketAddrV4::new(Ipv4Addr::UNSPECIFIED, 1900);
        let multicast_ip = Ipv4Addr::new(239, 255, 255, 250);

        if let Ok(socket) = UdpSocket::bind(bind_addr).await {
            let _ = socket.join_multicast_v4(multicast_ip, Ipv4Addr::UNSPECIFIED);

            let mut buf = [0u8; 4096];
            while let Ok((len, from)) = socket.recv_from(&mut buf).await {
                if let std::net::SocketAddr::V4(v4) = from {
                    let packet = &buf[..len];
                    if let Some(media) = parse_ssdp_packet(*v4.ip(), packet) {
                        if ssdp_tx.send(media).await.is_err() {
                            break;
                        }
                    }
                }
            }
        }
    });
}

/// Parse mDNS payload to find services like Google Cast, Spotify, AirPlay
fn parse_mdns_packet(ip: Ipv4Addr, data: &[u8]) -> Option<DiscoveredMedia> {
    let s = String::from_utf8_lossy(data);

    if s.contains("_googlecast._tcp") {
        // Extract friendly device name if present in TXT record (fn=Name)
        let fn_tag = s.split("fn=").nth(1).and_then(|part| part.split('\0').next());
        return Some(DiscoveredMedia {
            ip,
            service: "Chromecast / Google Cast".into(),
            detail: fn_tag.map(|n| n.trim().to_string()),
        });
    }

    if s.contains("_spotify-connect._tcp") {
        return Some(DiscoveredMedia {
            ip,
            service: "Spotify Connect".into(),
            detail: None,
        });
    }

    if s.contains("_airplay._tcp") || s.contains("_raop._tcp") {
        return Some(DiscoveredMedia {
            ip,
            service: "Apple AirPlay".into(),
            detail: None,
        });
    }

    None
}

/// Parse SSDP UPnP NOTIFY or M-SEARCH messages
fn parse_ssdp_packet(ip: Ipv4Addr, data: &[u8]) -> Option<DiscoveredMedia> {
    let s = String::from_utf8_lossy(data);

    if s.contains("MediaRenderer") || s.contains("dial-multiscreen-org") {
        // Smart TV / Cast DIAL protocol (YouTube / Netflix casting)
        return Some(DiscoveredMedia {
            ip,
            service: "Smart TV Media Stream".into(),
            detail: None,
        });
    }

    None
}
