/// WiFi Wave — Animated Network Device Visualizer & Traffic Inspector
///
/// Entry point: sets up the terminal, launches the network scanner,
/// raw packet sniffer, and multicast discovery in background tokio tasks,
/// and runs the TUI event loop at 30 FPS.

mod alerts;
mod animation;
mod app;
mod classifier;
mod discovery;
mod scanner;
mod sniffer;
mod ui;
mod vendor;

use std::io;
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use crossterm::ExecutableCommand;
use ratatui::prelude::CrosstermBackend;
use ratatui::Terminal;
use tokio::sync::mpsc;

use app::{App, AppAction};
use discovery::DiscoveredMedia;
use scanner::Device;
use sniffer::PacketActivity;

/// Target frame rate for the animation (30 FPS).
const TICK_MS: u64 = 33;
/// How long between automatic rescans (seconds).
const RESCAN_INTERVAL_SECS: u64 = 15;

#[tokio::main]
async fn main() -> io::Result<()> {
    // ── Detect network interface ─────────────────────────────────────
    let interface = match scanner::detect_interface() {
        Ok(info) => info,
        Err(e) => {
            eprintln!("❌ Error: {e}");
            eprintln!();
            eprintln!("Make sure you are connected to a network.");
            eprintln!("Usage: sudo wifi-wave");
            std::process::exit(1);
        }
    };

    println!("📡 WiFi Wave v3 — scanning on {} ({})", interface.name, interface.ip);
    println!("   Subnet: {}", interface.network);
    if let Some(gw) = interface.gateway {
        println!("   Gateway: {gw}");
    }
    println!("   Measuring proximity & launching traffic inspector...");

    // ── Channels ─────────────────────────────────────────────────────
    let (scan_tx, mut scan_rx) = mpsc::channel::<Vec<Device>>(4);
    let (pkt_tx, mut pkt_rx) = mpsc::channel::<PacketActivity>(64);
    let (media_tx, mut media_rx) = mpsc::channel::<DiscoveredMedia>(32);

    // ── Initial full scan ────────────────────────────────────────────
    let initial_devices = scanner::full_scan(&interface).await;
    println!("   Found {} devices. Launching TUI...", initial_devices.len());

    // ── Start Packet Sniffer (DNS & TLS SNI) ─────────────────────────
    let sniffer_iface = interface.name.clone();
    tokio::spawn(async move {
        sniffer::start_sniffer(sniffer_iface, pkt_tx).await;
    });

    // ── Start Multicast Streaming Discovery (mDNS & SSDP) ────────────
    tokio::spawn(async move {
        discovery::start_discovery(media_tx).await;
    });

    // ── Setup terminal ──────────────────────────────────────────────
    enable_raw_mode()?;
    io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    // ── Initialize app state ─────────────────────────────────────────
    let mut app = App::new(interface.clone());
    app.wifi_signal = scanner::get_wifi_signal();
    app.update_devices(initial_devices);

    // ── Spawn background periodic scanner task ───────────────────────
    let scan_interface = interface.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(RESCAN_INTERVAL_SECS)).await;
            let devices = scanner::full_scan(&scan_interface).await;
            if scan_tx.send(devices).await.is_err() {
                break;
            }
        }
    });

    // ── Trigger rescan channel (for manual rescan with 'r' key) ──────
    let (rescan_trigger_tx, mut rescan_trigger_rx) = mpsc::channel::<()>(1);

    let rescan_interface = interface.clone();
    let rescan_scan_tx = {
        let (tx, rx) = mpsc::channel::<Vec<Device>>(4);
        let rescan_iface = rescan_interface.clone();
        tokio::spawn(async move {
            while rescan_trigger_rx.recv().await.is_some() {
                let devices = scanner::full_scan(&rescan_iface).await;
                if tx.send(devices).await.is_err() {
                    break;
                }
            }
        });
        rx
    };
    let mut manual_scan_rx = rescan_scan_tx;

    // ── Main event loop ──────────────────────────────────────────────
    let tick_duration = Duration::from_millis(TICK_MS);

    loop {
        if !app.running {
            break;
        }

        // Draw TUI
        terminal.draw(|frame| {
            ui::draw(frame, &app);
        })?;

        // Advance animation & trackers
        app.tick();

        // Handle keyboard events
        if event::poll(tick_duration)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    match app.handle_key(key) {
                        AppAction::Quit => break,
                        AppAction::ForceRescan => {
                            let _ = rescan_trigger_tx.try_send(());
                        }
                        AppAction::SaveKnownDevices => {
                            let _ = app.save_known_devices();
                        }
                        AppAction::None => {}
                    }
                }
            }
        }

        // Check for scan results
        while let Ok(devices) = scan_rx.try_recv() {
            app.update_devices(devices);
        }
        while let Ok(devices) = manual_scan_rx.try_recv() {
            app.update_devices(devices);
        }

        // Check for captured packet activity (DNS / TLS SNI)
        while let Ok(pkt) = pkt_rx.try_recv() {
            app.record_packet_activity(pkt);
        }

        // Check for discovered streaming media (Chromecast / Spotify / AirPlay)
        while let Ok(media) = media_rx.try_recv() {
            app.record_media_activity(media);
        }
    }

    // ── Restore terminal ─────────────────────────────────────────────
    disable_raw_mode()?;
    io::stdout().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    println!("👋 WiFi Wave exited. Discovered {} devices.", app.devices.len());
    Ok(())
}
