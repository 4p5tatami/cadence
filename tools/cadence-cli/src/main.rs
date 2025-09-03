use anyhow::Result;
use cadence_core::Player;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "cadence", version, about = "Cadence CLI (MVP)")]
struct Cli {
    #[command(subcommand)]
    cmd: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Play a file (FLAC/WAV/etc.)
    Play { path: String },
    /// Pause playback
    Pause,
    /// Resume playback
    Resume,
    /// Stop playback
    Stop,
    /// Seek approximately to ms in the same file
    Seek { path: String, to_ms: u64 },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let player = Player::new_default()?;

    match cli.cmd {
        Commands::Play { path } => {
            let info = player.load_and_play(&path)?;
            println!("Playing: {}", info.path);
            if let Some(d) = info.duration_ms {
                println!("Duration ~ {} ms", d);
            }
            // Keep process alive while playing; crude loop for MVP.
            while !player.empty() && !player.is_paused() {
                std::thread::sleep(std::time::Duration::from_millis(200));
            }
        }
        Commands::Pause => player.pause(),
        Commands::Resume => player.resume(),
        Commands::Stop => player.stop(),
        Commands::Seek { path, to_ms } => {
            player.seek_approx(&path, to_ms)?;
            println!("Seeked to {} ms", to_ms);
        }
    }

    Ok(())
}
