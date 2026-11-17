#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod player;
mod ui;
mod websocket;

use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, UdpSocket};
use std::process::ExitCode;
use std::sync::Arc;

use cadence_core::Library;
use mdns_sd::{ServiceDaemon, ServiceInfo};
use tokio::net::TcpListener;

const PORT: u16 = 7878;

fn main() -> ExitCode {
    if std::env::args_os()
        .nth(1)
        .is_some_and(|arg| arg == "--version")
    {
        let version = option_env!("CADENCE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"));
        println!("Cadence {version}");
        return ExitCode::SUCCESS;
    }

    let app = fltk::app::App::default();
    match run(&app) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let message = format!("Could not start Cadence.\n{error:#}");
            eprintln!("{message}");
            fltk::dialog::alert_default(&message);
            ExitCode::FAILURE
        }
    }
}

fn run(app: &fltk::app::App) -> Result<(), Box<dyn Error>> {
    // Keep the same data path so upgrades can open existing library indexes.
    let data_dir = dirs::data_dir().ok_or("Could not locate the application data directory")?;
    let library = Arc::new(Library::open(
        &data_dir.join("com.cadence.desktop/cadence.db"),
    )?);
    let runtime = tokio::runtime::Runtime::new()?;
    let listener = runtime
        .block_on(TcpListener::bind((Ipv4Addr::UNSPECIFIED, PORT)))
        .map_err(|error| format!("Could not listen on port {PORT}. {error}"))?;
    let player = player::spawn(Arc::clone(&library));
    let (sender, receiver) = fltk::app::channel();
    let mut last_label = String::new();
    runtime.spawn(websocket::serve(
        listener,
        player,
        Arc::clone(&library),
        move |connected, status| {
            let label = ui::playback_label(connected, status);
            if label != last_label {
                last_label.clone_from(&label);
                sender.send(ui::Message::Playback(label));
            }
        },
    ));

    let ip = lan_ip().ok();
    let mdns = ip.and_then(|ip| advertise_on_lan(ip).ok());
    let address = format!(
        "ws://{}:{PORT}",
        ip.unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST))
    );
    ui::run(app, library, &address, sender, receiver);
    if let Some(mdns) = mdns {
        let _ = mdns.shutdown();
    }
    runtime.shutdown_background();
    Ok(())
}

fn lan_ip() -> std::io::Result<IpAddr> {
    // Connecting a UDP socket selects the local route without sending a packet.
    let socket = UdpSocket::bind((Ipv4Addr::UNSPECIFIED, 0))?;
    socket.connect((Ipv4Addr::new(8, 8, 8, 8), 80))?;
    Ok(socket.local_addr()?.ip())
}

fn advertise_on_lan(ip: IpAddr) -> Result<ServiceDaemon, Box<dyn Error>> {
    let hostname = gethostname::gethostname().to_string_lossy().into_owned();
    let service = ServiceInfo::new(
        "_cadence._tcp.local.",
        &hostname,
        &format!("{hostname}.local."),
        ip,
        PORT,
        None,
    )?;
    let mdns = ServiceDaemon::new()?;
    mdns.register(service)?;
    Ok(mdns)
}
