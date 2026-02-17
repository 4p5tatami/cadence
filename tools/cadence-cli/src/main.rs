use anyhow::Result;
use cadence_core::Player;
use clap::Parser;
use std::io::{self, BufRead, Write};

#[derive(Parser)]
#[command(name = "cadence", version, about = "Cadence CLI (MVP)")]
struct Cli {
    /// Audio file to play
    path: String,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let player = Player::new()?;

    // Play the file
    let info = player.load_and_play(&cli.path)?;
    println!(
        "Playing: {} ({} ms)",
        info.path,
        info.duration_ms.unwrap_or(0)
    );
    println!("Commands: pause, resume, stop, seek <ms>, quit");

    // REPL loop for commands
    let stdin = io::stdin();
    print!("> ");
    io::stdout().flush()?;

    for line in stdin.lock().lines() {
        let line = line?;
        let input = line.trim();
        let parts: Vec<&str> = input.split_whitespace().collect();

        if parts.is_empty() {
            print!("> ");
            io::stdout().flush()?;
            continue;
        }

        match parts[0] {
            "pause" => {
                player.pause();
                println!("Paused");
            }
            "resume" => {
                player.resume();
                println!("Resumed");
            }
            "stop" => {
                player.stop();
                println!("Stopped");
            }
            "seek" => {
                if parts.len() < 2 {
                    println!("Usage: seek <milliseconds>");
                } else {
                    match parts[1].parse::<u64>() {
                        Ok(ms) => {
                            if let Err(e) = player.seek_approx(&cli.path, ms) {
                                println!("Seek error: {}", e);
                            } else {
                                println!("Seeked to {} ms", ms);
                            }
                        }
                        Err(_) => println!("Invalid number: {}", parts[1]),
                    }
                }
            }
            "quit" | "q" | "exit" => {
                player.stop();
                break;
            }
            "help" | "h" => {
                println!("Commands: pause, resume, stop, seek <ms>, quit");
            }
            _ => {
                println!("Unknown command: {}. Type 'help' for commands.", parts[0]);
            }
        }

        print!("> ");
        io::stdout().flush()?;
    }

    Ok(())
}
