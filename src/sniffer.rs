/// Packet sniffer module.
/// Uses Linux raw sockets to inspect DNS (UDP 53) and TLS SNI (TCP 443)
/// packets in real time to discover which websites each device is visiting.

use std::net::Ipv4Addr;
use tokio::sync::mpsc::Sender;

#[derive(Debug, Clone)]
pub struct PacketActivity {
    pub ip: Ipv4Addr,
    pub domain: String,
    pub protocol: &'static str,
}

/// Start the background raw packet sniffer on the given interface.
pub async fn start_sniffer(iface_name: String, tx: Sender<PacketActivity>) {
    // Run socket loop in a dedicated blocking thread because raw sockets use blocking recv
    tokio::task::spawn_blocking(move || {
        let fd = unsafe {
            // ETH_P_IP is 0x0800 in host order, 0x0008 in big endian / network order
            let proto = (libc::ETH_P_IP as u16).to_be() as i32;
            libc::socket(libc::AF_PACKET, libc::SOCK_RAW, proto)
        };

        if fd < 0 {
            // Raw socket requires root or CAP_NET_RAW. If unavailable, exit gracefully.
            return;
        }

        // Bind to interface
        let ifindex = unsafe {
            let c_iface = std::ffi::CString::new(iface_name.as_str()).unwrap();
            libc::if_nametoindex(c_iface.as_ptr())
        };

        if ifindex > 0 {
            let mut addr: libc::sockaddr_ll = unsafe { std::mem::zeroed() };
            addr.sll_family = libc::AF_PACKET as u16;
            addr.sll_protocol = (libc::ETH_P_IP as u16).to_be();
            addr.sll_ifindex = ifindex as i32;

            unsafe {
                libc::bind(
                    fd,
                    &addr as *const libc::sockaddr_ll as *const libc::sockaddr,
                    std::mem::size_of::<libc::sockaddr_ll>() as libc::socklen_t,
                );
            }
        }

        let mut buf = [0u8; 2048];
        loop {
            let n = unsafe { libc::recv(fd, buf.as_mut_ptr() as *mut libc::c_void, buf.len(), 0) };
            if n <= 0 {
                break;
            }

            let packet = &buf[..n as usize];
            if let Some(act) = parse_ethernet_packet(packet) {
                if tx.blocking_send(act).is_err() {
                    break;
                }
            }
        }

        unsafe {
            libc::close(fd);
        }
    });
}

/// Parse Ethernet frame -> IPv4 -> UDP/TCP -> DNS / TLS SNI
fn parse_ethernet_packet(packet: &[u8]) -> Option<PacketActivity> {
    // Ethernet header is 14 bytes
    if packet.len() < 14 + 20 {
        return None;
    }

    let ethertype = u16::from_be_bytes([packet[12], packet[13]]);
    if ethertype != 0x0800 {
        // Not IPv4
        return None;
    }

    let ip_data = &packet[14..];
    let version_ihl = ip_data[0];
    let ihl = (version_ihl & 0x0F) as usize * 4;
    if ip_data.len() < ihl || ihl < 20 {
        return None;
    }

    let protocol = ip_data[9];
    let src_ip = Ipv4Addr::new(ip_data[12], ip_data[13], ip_data[14], ip_data[15]);
    let dst_ip = Ipv4Addr::new(ip_data[16], ip_data[17], ip_data[18], ip_data[19]);

    let payload = &ip_data[ihl..];

    // ── UDP (Protocol 17) ──
    if protocol == 17 && payload.len() >= 8 {
        let src_port = u16::from_be_bytes([payload[0], payload[1]]);
        let dst_port = u16::from_be_bytes([payload[2], payload[3]]);
        let udp_data = &payload[8..];

        // DNS Query or Response (Port 53)
        if dst_port == 53 || src_port == 53 {
            if let Some(domain) = parse_dns_qname(udp_data) {
                // If it's a request to port 53, the client is src_ip
                // If it's a response from port 53, the client is dst_ip
                let client_ip = if dst_port == 53 { src_ip } else { dst_ip };
                return Some(PacketActivity {
                    ip: client_ip,
                    domain,
                    protocol: "DNS",
                });
            }
        }
    }

    // ── TCP (Protocol 6) ──
    if protocol == 6 && payload.len() >= 20 {
        let dst_port = u16::from_be_bytes([payload[2], payload[3]]);
        let data_offset = ((payload[12] >> 4) & 0x0F) as usize * 4;
        if payload.len() >= data_offset {
            let tcp_data = &payload[data_offset..];
            // TLS Handshake on port 443
            if dst_port == 443 && !tcp_data.is_empty() {
                if let Some(sni) = parse_tls_sni(tcp_data) {
                    return Some(PacketActivity {
                        ip: src_ip,
                        domain: sni,
                        protocol: "HTTPS/SNI",
                    });
                }
            }
        }
    }

    None
}

/// Extract first QNAME from DNS payload
fn parse_dns_qname(data: &[u8]) -> Option<String> {
    if data.len() < 12 {
        return None;
    }

    let qdcount = u16::from_be_bytes([data[4], data[5]]);
    if qdcount == 0 {
        return None;
    }

    let mut pos = 12;
    let mut labels = Vec::new();

    while pos < data.len() {
        let len = data[pos] as usize;
        if len == 0 {
            break;
        }
        // Compressed pointer
        if (len & 0xC0) == 0xC0 {
            break;
        }
        pos += 1;
        if pos + len > data.len() {
            return None;
        }

        let label = std::str::from_utf8(&data[pos..pos + len]).ok()?;
        labels.push(label);
        pos += len;
    }

    if labels.is_empty() {
        None
    } else {
        Some(labels.join("."))
    }
}

/// Extract Server Name Indication (SNI) from TLS ClientHello
fn parse_tls_sni(data: &[u8]) -> Option<String> {
    // ContentType == 22 (Handshake), Version >= 3.1
    if data.len() < 5 || data[0] != 0x16 {
        return None;
    }

    let record_len = u16::from_be_bytes([data[3], data[4]]) as usize;
    if data.len() < 5 + record_len || record_len < 4 {
        return None;
    }

    let handshake = &data[5..5 + record_len];
    // HandshakeType == 1 (ClientHello)
    if handshake[0] != 0x01 {
        return None;
    }

    // Skip Handshake Header (4 bytes) + Client Version (2 bytes) + Random (32 bytes)
    let mut pos = 38;
    if pos >= handshake.len() {
        return None;
    }

    // Session ID Length
    let sid_len = handshake[pos] as usize;
    pos += 1 + sid_len;

    // Cipher Suites Length
    if pos + 2 > handshake.len() {
        return None;
    }
    let cs_len = u16::from_be_bytes([handshake[pos], handshake[pos + 1]]) as usize;
    pos += 2 + cs_len;

    // Compression Methods Length
    if pos + 1 > handshake.len() {
        return None;
    }
    let comp_len = handshake[pos] as usize;
    pos += 1 + comp_len;

    // Extensions Length
    if pos + 2 > handshake.len() {
        return None;
    }
    let ext_len = u16::from_be_bytes([handshake[pos], handshake[pos + 1]]) as usize;
    pos += 2;

    let ext_end = (pos + ext_len).min(handshake.len());

    // Iterate extensions to find Server Name (type 0x0000)
    while pos + 4 <= ext_end {
        let ext_type = u16::from_be_bytes([handshake[pos], handshake[pos + 1]]);
        let ext_data_len = u16::from_be_bytes([handshake[pos + 2], handshake[pos + 3]]) as usize;
        pos += 4;

        if ext_type == 0x0000 {
            // Server Name Extension
            if pos + 2 <= ext_end {
                let _server_name_list_len = u16::from_be_bytes([handshake[pos], handshake[pos + 1]]) as usize;
                pos += 2;
                if pos + 3 <= ext_end {
                    let name_type = handshake[pos];
                    let name_len = u16::from_be_bytes([handshake[pos + 1], handshake[pos + 2]]) as usize;
                    pos += 3;
                    if name_type == 0 && pos + name_len <= ext_end {
                        let hostname = std::str::from_utf8(&handshake[pos..pos + name_len]).ok()?;
                        return Some(hostname.to_string());
                    }
                }
            }
            return None;
        }

        pos += ext_data_len;
    }

    None
}
