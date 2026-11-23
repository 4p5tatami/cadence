#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod display;
mod folder_picker;
mod player;
mod terminal;
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
    let command = std::env::args_os().nth(1);
    if command.as_deref().is_some_and(|arg| arg == "--version") {
        let version = option_env!("CADENCE_VERSION").unwrap_or(env!("CARGO_PKG_VERSION"));
        println!("Cadence {version}");
        return ExitCode::SUCCESS;
    }

    let terminal = command.as_deref().is_some_and(|arg| arg == "--terminal");
    let app = (!terminal).then(fltk::app::App::default);
    match run(app.as_ref()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            let message = format!("Could not start Cadence.\n{error:#}");
            eprintln!("{message}");
            if app.is_some() {
                fltk::dialog::alert_default(&message);
            }
            ExitCode::FAILURE
        }
    }
}

fn run(app: Option<&fltk::app::App>) -> Result<(), Box<dyn Error>> {
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
    let ip = lan_ip().ok();
    let mdns = ip.and_then(|ip| advertise_on_lan(ip).ok());
    let address_msg = format!(
        "Websocket listening on ws://{}:{PORT}",
        ip.unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST))
    );
    let result = if let Some(app) = app {
        let (sender, receiver) = fltk::app::channel();
        let mut last_playback = None;
        runtime.spawn(websocket::serve(
            listener,
            player,
            Arc::clone(&library),
            move |connected, status| {
                let playback = ui::PlaybackDisplay::new(connected, status);
                if last_playback.as_ref() != Some(&playback) {
                    last_playback = Some(playback.clone());
                    sender.send(ui::Message::Playback(playback));
                }
            },
        ));
        ui::run(app, library, &address_msg, sender, receiver);
        Ok(())
    } else {
        runtime.block_on(terminal::run(listener, player, library, &address_msg))
    };
    if let Some(mdns) = mdns {
        let _ = mdns.shutdown();
    }
    runtime.shutdown_background();
    result.map_err(Into::into)
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
