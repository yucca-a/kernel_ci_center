//! kci — Yucca kernel CI orchestrator for Samsung sm85xx/sm87xx GKI kernels.
//!
//! Drives the per-device kernel `build.sh` (feature application + make) from a
//! single typed config, collects the packed zip, and optionally publishes a
//! GitHub Release. The heavy lifting (make/clang/patch) stays in the kernel
//! repos; this is the orchestration layer.

mod config;
mod pipeline;
mod release;
mod util;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "kci", version, about = "Yucca kernel CI orchestrator")]
struct Cli {
    /// Path to the device config.
    #[arg(long, default_value = "config/devices.toml", global = true)]
    config: String,
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// List configured devices and their modes.
    List,
    /// Build one device in one mode (optionally release).
    Build {
        /// Device id (e.g. sm8750).
        #[arg(long)]
        device: String,
        /// Build mode.
        #[arg(long, default_value = "resukisu")]
        mode: String,
        /// Publish the zip as a GitHub Release.
        #[arg(long)]
        release: bool,
        /// Print the plan without building.
        #[arg(long)]
        dry_run: bool,
        /// Working directory for clones and artifacts.
        #[arg(long, default_value = "work")]
        work: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg = config::load(&cli.config).with_context(|| format!("loading {}", cli.config))?;

    match cli.cmd {
        Cmd::List => {
            println!("Configured devices ({}):", cfg.devices.len());
            for d in &cfg.devices {
                println!(
                    "  {:<8} {:<10} modes={:<22} {} @ {}",
                    d.id,
                    if d.enabled { "[enabled]" } else { "[disabled]" },
                    format!("{:?}", d.modes),
                    d.repo,
                    d.branch
                );
            }
            println!("kmi_generation default = {}", cfg.defaults.kmi_generation);
        }
        Cmd::Build {
            device,
            mode,
            release,
            dry_run,
            work,
        } => {
            pipeline::run_build(&cfg, &device, &mode, &work, release, dry_run)?;
        }
    }
    Ok(())
}
