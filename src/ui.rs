/// TUI rendering module.
/// Draws the wave animation canvas, distance-scaled device icons with active service badges,
/// real-time website activity feed, live bandwidth, WiFi signal meter, and detail panel.

use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::app::{format_bandwidth, App, ScanStatus};
use crate::scanner::Device;

// ─── Main render function ────────────────────────────────────────────

pub fn draw(frame: &mut Frame, app: &App) {
    let size = frame.area();

    // Layout: Header (3) + Main Canvas (Flex) + Activity Feed (4) + Footer (3)
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // header
            Constraint::Min(8),    // canvas
            Constraint::Length(4), // activity feed
            Constraint::Length(3), // footer
        ])
        .split(size);

    draw_header(frame, chunks[0], app);
    draw_canvas(frame, chunks[1], app);
    draw_activity_feed(frame, chunks[2], app);
    draw_footer(frame, chunks[3], app);

    // If a device is selected, draw the detail panel overlay
    if let Some(sel_idx) = app.selected {
        if let Some(device) = app.devices.get(sel_idx) {
            draw_detail_panel(frame, chunks[1], device, app);
        }
    }
}

// ─── Header ──────────────────────────────────────────────────────────

fn draw_header(frame: &mut Frame, area: Rect, app: &App) {
    let mut spans = vec![
        Span::styled(
            " 📡 WiFi Wave v3  ",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("│  Interface: {} ({})  ", app.interface.name, app.interface.ip),
            Style::default().fg(Color::White),
        ),
    ];

    if let Some(ref sig) = app.wifi_signal {
        let sig_color = match sig.quality {
            crate::scanner::SignalQuality::Excellent | crate::scanner::SignalQuality::Good => Color::Green,
            crate::scanner::SignalQuality::Fair => Color::Yellow,
            _ => Color::Red,
        };
        spans.push(Span::styled(
            format!("│  WiFi: {} {} ({} dBm, ~{:.1}m)  ", sig.quality.bars(), sig.quality.label(), sig.rssi_dbm, sig.distance_m),
            Style::default().fg(sig_color),
        ));
    } else {
        spans.push(Span::styled(
            "│  Mode: Ethernet/LAN  ",
            Style::default().fg(Color::DarkGray),
        ));
    }

    if app.alerts.has_alerts() {
        let alert_style = if app.alerts.flash_visible {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::DarkGray)
        };
        spans.push(Span::styled(
            format!("│  🚨 {} NEW DEVICE(S)! ", app.alerts.new_count),
            alert_style,
        ));
    }

    let header = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(if app.alerts.has_alerts() { Color::Red } else { Color::DarkGray })),
    );
    frame.render_widget(header, area);
}

// ─── Real-Time Activity Feed ─────────────────────────────────────────

fn draw_activity_feed(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" 🌐 Live Network & Website Traffic Feed ");

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height < 1 || app.activity_feed.is_empty() {
        let empty_msg = Paragraph::new(Line::from(vec![
            Span::styled(" Listening for DNS & HTTPS website requests across the network...", Style::default().fg(Color::DarkGray)),
        ]));
        frame.render_widget(empty_msg, inner);
        return;
    }

    let mut lines = Vec::new();
    for item in app.activity_feed.iter().take(inner.height as usize) {
        let cat_icon = item.category.icon();
        let cat_color = match item.category {
            crate::classifier::ServiceCategory::VideoStreaming => Color::Red,
            crate::classifier::ServiceCategory::SocialChat => Color::Magenta,
            crate::classifier::ServiceCategory::MusicStreaming => Color::Green,
            crate::classifier::ServiceCategory::Gaming => Color::Yellow,
            crate::classifier::ServiceCategory::WorkCloud => Color::Cyan,
            crate::classifier::ServiceCategory::WebBrowsing => Color::Blue,
            crate::classifier::ServiceCategory::SystemOther => Color::DarkGray,
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {}  ", item.ip), Style::default().fg(Color::White)),
            Span::styled("→ ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!("{:<30} ", item.domain), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
            Span::styled(format!("({} {}) ", cat_icon, item.service_name), Style::default().fg(cat_color)),
            Span::styled(format!("[{}]", item.protocol), Style::default().fg(Color::DarkGray)),
        ]));
    }

    let feed_widget = Paragraph::new(lines);
    frame.render_widget(feed_widget, inner);
}

// ─── Footer / status bar ─────────────────────────────────────────────

fn draw_footer(frame: &mut Frame, area: Rect, app: &App) {
    let status_text = match &app.scan_status {
        ScanStatus::Scanning => "⟳ Scanning...".to_string(),
        ScanStatus::Done { device_count } => format!("✓ {} devices", device_count),
    };

    let gateway_text = match app.interface.gateway {
        Some(gw) => format!("GW: {gw}"),
        None => "GW: none".to_string(),
    };

    let rx_str = format_bandwidth(app.bandwidth.current_rx);
    let tx_str = format_bandwidth(app.bandwidth.current_tx);
    let bw_text = format!("▼ {rx_str}  ▲ {tx_str}");

    let mut spans = vec![
        Span::styled(format!(" {status_text} │ {gateway_text} │ "), Style::default().fg(Color::DarkGray)),
        Span::styled(format!("{bw_text} "), Style::default().fg(Color::Green)),
        Span::styled("│ ↑↓ Select │ s Save Known │ r Rescan │ q Quit ", Style::default().fg(Color::DarkGray)),
    ];

    if app.alerts.has_alerts() {
        spans.push(Span::styled("(Press 's' to whitelist)", Style::default().fg(Color::Yellow)));
    }

    let footer = Paragraph::new(Line::from(spans)).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(footer, area);
}

// ─── Main canvas: waves + devices ────────────────────────────────────

fn draw_canvas(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Network Radar & Live Activities ");
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.width < 4 || inner.height < 4 {
        return;
    }

    let buf = frame.buffer_mut();

    let cx = inner.x as f64 + inner.width as f64 / 2.0;
    let cy = inner.y as f64 + inner.height as f64 / 2.0;

    // 1. Draw wave rings
    draw_waves(buf, inner, app, cx, cy);

    // 2. Draw connection lines
    let device_positions = compute_device_positions(&app.devices, cx, cy, inner);
    draw_connections(buf, inner, app, cx, cy, &device_positions);

    // 3. Draw router icon at center
    draw_router(buf, inner, cx as u16, cy as u16, app);

    // 4. Draw device icons with distance & active service badges
    draw_devices(buf, inner, app, &device_positions);
}

// ─── Wave rings ──────────────────────────────────────────────────────

fn draw_waves(buf: &mut Buffer, area: Rect, app: &App, cx: f64, cy: f64) {
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            if let Some((ch, color)) = app.waves.get_wave_at(x as f64, y as f64, cx, cy) {
                let cell = &mut buf[(x, y)];
                if cell.symbol() == " " {
                    cell.set_char(ch);
                    cell.set_fg(color);
                }
            }
        }
    }
}

// ─── Device positioning with Distance scaling ────────────────────────

fn compute_device_positions(
    devices: &[Device],
    cx: f64,
    cy: f64,
    area: Rect,
) -> Vec<(u16, u16)> {
    if devices.is_empty() {
        return vec![];
    }

    let count = devices.len();
    let base_max_rx = (area.width as f64) * 0.40;
    let base_max_ry = (area.height as f64) * 0.40;

    (0..count)
        .map(|i| {
            let dev = &devices[i];
            let dist = dev.estimated_distance_m.unwrap_or(5.0);
            let norm = ((dist - 1.0) / 11.0).clamp(0.0, 1.0);
            let dist_scale = 0.45 + 0.50 * norm;

            let rx = base_max_rx * dist_scale;
            let ry = base_max_ry * dist_scale;

            let angle = (i as f64 / count as f64) * std::f64::consts::TAU
                - std::f64::consts::FRAC_PI_2;

            let px = cx + rx * angle.cos();
            let py = cy + ry * angle.sin();

            let px = px.clamp(area.x as f64 + 2.0, (area.x + area.width) as f64 - 20.0);
            let py = py.clamp(area.y as f64 + 1.0, (area.y + area.height) as f64 - 2.0);

            (px as u16, py as u16)
        })
        .collect()
}

// ─── Connection lines ────────────────────────────────────────────────

fn draw_connections(
    buf: &mut Buffer,
    area: Rect,
    app: &App,
    cx: f64,
    cy: f64,
    positions: &[(u16, u16)],
) {
    for (i, &(dx, dy)) in positions.iter().enumerate() {
        draw_line(buf, area, app, cx as u16, cy as u16, dx, dy, i);
    }
}

fn draw_line(
    buf: &mut Buffer,
    area: Rect,
    app: &App,
    x0: u16,
    y0: u16,
    x1: u16,
    y1: u16,
    device_idx: usize,
) {
    let steps = ((x1 as f64 - x0 as f64).abs())
        .max((y1 as f64 - y0 as f64).abs()) as usize;
    if steps == 0 {
        return;
    }

    let is_new = app.devices.get(device_idx)
        .map(|d| app.alerts.is_new(&d.mac))
        .unwrap_or(false);

    for step in 0..=steps {
        let t = step as f64 / steps as f64;
        let x = (x0 as f64 + (x1 as f64 - x0 as f64) * t) as u16;
        let y = (y0 as f64 + (y1 as f64 - y0 as f64) * t) as u16;

        if (step < 2) || (step > steps - 3) {
            continue;
        }

        if x >= area.x && x < area.x + area.width && y >= area.y && y < area.y + area.height {
            let cell = &mut buf[(x, y)];
            if let Some(dot_ch) = app.conn_anim.get_dot_at(t) {
                cell.set_char(dot_ch);
                cell.set_fg(if is_new { Color::Red } else { Color::Green });
            } else if cell.symbol() == " " {
                cell.set_char('·');
                cell.set_fg(Color::Rgb(40, 40, 40));
            }
        }
    }
}

// ─── Router icon ─────────────────────────────────────────────────────

fn draw_router(buf: &mut Buffer, area: Rect, cx: u16, cy: u16, app: &App) {
    let label = match app.interface.gateway {
        Some(gw) => format!("🌐 {gw}"),
        None => "🌐 Router".to_string(),
    };

    let label_x = cx.saturating_sub(label.len() as u16 / 2);
    if cy >= area.y && cy < area.y + area.height {
        write_str(buf, area, label_x, cy, &label, Color::Yellow, true);
    }
}

// ─── Device icons with Activity Badges ────────────────────────────────

fn draw_devices(
    buf: &mut Buffer,
    area: Rect,
    app: &App,
    positions: &[(u16, u16)],
) {
    for (i, (device, &(px, py))) in app.devices.iter().zip(positions.iter()).enumerate() {
        let is_selected = app.selected == Some(i);
        let is_new = app.alerts.is_new(&device.mac);

        let ip_str = device.ip.to_string();
        let name = device
            .hostname
            .as_deref()
            .unwrap_or(&ip_str);

        let dist_str = match (device.latency_ms, device.estimated_distance_m) {
            (Some(lat), Some(dist)) => format!(" ({:.1}ms ~{:.1}m)", lat, dist),
            (Some(lat), None) => format!(" ({:.1}ms)", lat),
            _ => String::new(),
        };

        // Active service badge (e.g. [🎬 YouTube])
        let service_badge = if let Some(act) = app.device_activities.get(&device.ip) {
            if let Some(last_svc) = act.active_services.last() {
                format!(" [{}]", last_svc)
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        let status_badge = if is_new {
            if app.alerts.flash_visible { " ⚠️ NEW" } else { "       " }
        } else {
            " ✓"
        };

        let label = format!("{} {}{}{}{}", device.icon, name, dist_str, service_badge, status_badge);

        let color = if is_selected {
            Color::White
        } else if is_new {
            Color::LightRed
        } else if device.is_gateway {
            Color::Yellow
        } else {
            Color::Green
        };

        write_str(buf, area, px, py, &label, color, is_selected);

        if is_selected && py + 1 < area.y + area.height {
            let detail = format!("  {} ({})", device.mac, device.vendor);
            write_str(buf, area, px, py + 1, &detail, Color::Cyan, false);
        }
    }
}

// ─── Detail panel with Browsing / Streaming History ──────────────────

fn draw_detail_panel(frame: &mut Frame, canvas_area: Rect, device: &Device, app: &App) {
    let panel_w = 48u16.min(canvas_area.width.saturating_sub(4));
    let panel_h = 14u16.min(canvas_area.height.saturating_sub(2));

    let panel_area = Rect::new(
        canvas_area.x + canvas_area.width - panel_w - 1,
        canvas_area.y + canvas_area.height - panel_h - 1,
        panel_w,
        panel_h,
    );

    frame.render_widget(Clear, panel_area);

    let hostname = device
        .hostname
        .as_deref()
        .unwrap_or("(no hostname)");

    let elapsed = device.first_seen.elapsed();
    let seen_ago = if elapsed.as_secs() < 60 {
        format!("{}s ago", elapsed.as_secs())
    } else {
        format!("{}m ago", elapsed.as_secs() / 60)
    };

    let is_new = app.alerts.is_new(&device.mac);

    let lat_str = match device.latency_ms {
        Some(l) => format!("{:.2} ms", l),
        None => "N/A".to_string(),
    };

    let dist_str = match device.estimated_distance_m {
        Some(d) => format!("~{:.1} meters (via latency)", d),
        None => "Unknown".to_string(),
    };

    let mut lines = vec![
        Line::from(vec![
            Span::styled(" IP:         ", Style::default().fg(Color::DarkGray)),
            Span::styled(device.ip.to_string(), Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(" MAC:        ", Style::default().fg(Color::DarkGray)),
            Span::styled(&device.mac, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(" Hostname:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(hostname, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(" Vendor:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(&device.vendor, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled(" Latency:    ", Style::default().fg(Color::DarkGray)),
            Span::styled(lat_str, Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled(" Distance:   ", Style::default().fg(Color::DarkGray)),
            Span::styled(dist_str, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled(" Status:     ", Style::default().fg(Color::DarkGray)),
            Span::styled(
                if is_new { "⚠️ UNKNOWN / NEW DEVICE" } else { "✓ Whitelisted (Known)" },
                Style::default().fg(if is_new { Color::Red } else { Color::Green }).add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled(" First seen: ", Style::default().fg(Color::DarkGray)),
            Span::styled(format!(" {seen_ago}"), Style::default().fg(Color::White)),
        ]),
    ];

    // Add recent websites visited / active services
    if let Some(act) = app.device_activities.get(&device.ip) {
        if !act.active_services.is_empty() {
            let svcs = act.active_services.join(", ");
            lines.push(Line::from(vec![
                Span::styled(" Activities: ", Style::default().fg(Color::DarkGray)),
                Span::styled(svcs, Style::default().fg(Color::Yellow)),
            ]));
        }

        lines.push(Line::from(vec![
            Span::styled(" Top Visited Domains:", Style::default().fg(Color::DarkGray)),
        ]));

        for (domain, cat) in act.recent_domains.iter().take(3) {
            lines.push(Line::from(vec![
                Span::styled(format!("  • {} ", cat.icon()), Style::default().fg(Color::White)),
                Span::styled(domain.clone(), Style::default().fg(Color::Cyan)),
            ]));
        }
    } else {
        lines.push(Line::from(vec![
            Span::styled(" Activities: ", Style::default().fg(Color::DarkGray)),
            Span::styled("No DNS/Web traffic captured yet", Style::default().fg(Color::DarkGray)),
        ]));
    }

    let title = format!(" {} Device Activity & Info ", device.icon);
    let border_color = if is_new { Color::Red } else { Color::Cyan };
    let panel = Paragraph::new(lines).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color))
            .title(title),
    );

    frame.render_widget(panel, panel_area);
}

// ─── Helpers ─────────────────────────────────────────────────────────

fn write_str(
    buf: &mut Buffer,
    area: Rect,
    x: u16,
    y: u16,
    text: &str,
    color: Color,
    bold: bool,
) {
    if y < area.y || y >= area.y + area.height {
        return;
    }

    let style = if bold {
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(color)
    };

    let mut col = x;
    for ch in text.chars() {
        if col >= area.x + area.width {
            break;
        }
        if col >= area.x {
            let cell = &mut buf[(col, y)];
            cell.set_char(ch);
            cell.set_style(style);
        }
        let w = unicode_width(ch);
        col += w as u16;
    }
}

fn unicode_width(ch: char) -> usize {
    if ch.is_ascii() {
        1
    } else {
        2
    }
}
