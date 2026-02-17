use anyhow::Result;
use cadence_core::Player;
use clap::Parser;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Parser)]
#[command(name = "cadence", version, about = "Cadence CLI (MVP)")]
struct Cli {
    /// Audio file to play
    path: PathBuf,
}

/// Commands available in the REPL
#[derive(Debug, PartialEq)]
enum Command {
    Pause,
    Resume,
    Stop,
    Advance { seconds: i64 },
    Quit,
    Help,
}

impl FromStr for Command {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = input.split_whitespace().collect();

        if parts.is_empty() {
            return Err("Empty command".to_string());
        }

        match parts[0] {
            "pause" => Ok(Command::Pause),
            "resume" => Ok(Command::Resume),
            "stop" => Ok(Command::Stop),
            "+" | "-" => {
                if parts.len() < 2 {
                    Err("Usage: +/- <seconds>. Enter a number after +/-".to_string())
                } else {
                    let signed = format!("{}{}", parts[0], parts[1]);
                    match signed.parse::<i64>() {
                        Ok(seconds) => Ok(Command::Advance { seconds }),
                        Err(_) => Err(format!("Invalid number: {}", parts[1])),
                    }
                }
            }
            "quit" | "q" | "exit" => Ok(Command::Quit),
            "help" | "h" => Ok(Command::Help),
            cmd => Err(format!(
                "Unknown command: {}. Type 'help' for commands.",
                cmd
            )),
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let mut player = Player::new()?;

    // Play the file
    let info = player.load_and_play(cli.path.clone())?;
    println!(
        "Playing: {} ({} ms)",
        info.path.display(),
        info.duration_ms.unwrap_or(0)
    );

    let commands_description =
        "Commands: pause, resume, stop, +/- <seconds> (advance or rewind by <seconds>), quit";

    println!("{}", commands_description);

    // REPL loop for commands
    let stdin = io::stdin();
    print!("> ");
    io::stdout().flush()?;

    for line in stdin.lock().lines() {
        let line = line?;
        let input = line.trim();

        if input.is_empty() {
            print!("> ");
            io::stdout().flush()?;
            continue;
        }

        match input.parse::<Command>() {
            Ok(Command::Pause) => {
                player.pause();
                println!("Paused");
            }
            Ok(Command::Resume) => {
                player.resume();
                println!("Resumed");
            }
            Ok(Command::Stop) => {
                player.stop();
                println!("Stopped");
            }
            Ok(Command::Advance { seconds }) => {
                if let Err(e) = player.advance_or_rewind(seconds * 1000) {
                    println!("Error: {}", e);
                }
            }
            Ok(Command::Quit) => {
                player.stop();
                break;
            }
            Ok(Command::Help) => {
                println!("{}", commands_description);
            }
            Err(e) => {
                println!("{}", e);
            }
        }

        print!("> ");
        io::stdout().flush()?;
    }

    Ok(())
}
