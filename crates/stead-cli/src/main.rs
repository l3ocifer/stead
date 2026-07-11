use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "stead", version, about = "Whole-home localized mapping")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize a new site directory (georef + empty journal).
    Init {
        /// Site directory to create.
        path: std::path::PathBuf,
    },
    /// Replay the journal and print a site summary.
    Describe {
        /// Site directory.
        path: std::path::PathBuf,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Init { path } => {
            std::fs::create_dir_all(path.join("journal"))?;
            println!("initialized site at {}", path.display());
            println!("next: write georef.json (projected CRS + origin) — see docs/");
        }
        Command::Describe { path } => {
            let events = stead_core::Journal::replay(&path.join("journal"))?;
            println!(
                "site: {} — {} journaled events",
                path.display(),
                events.len()
            );
        }
    }
    Ok(())
}
