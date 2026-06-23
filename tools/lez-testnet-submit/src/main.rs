mod submit;

use std::{
    io::{self, Read as _},
    path::PathBuf,
};

use anyhow::{Context as _, Result};
use clap::{Parser, Subcommand};
use submit::{parse_payload_json, resolve_program_elf_path, submit_public_tx, SubmitResult};

#[derive(Parser)]
#[command(name = "lez-testnet-submit", about = "Step 18 rc3 public-tx submit helper")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Submit a generic public transaction (wallet JSON contract).
    SubmitPublicTx {
        #[arg(long)]
        wallet_config: PathBuf,
        #[arg(long)]
        wallet_storage: PathBuf,
        #[arg(long, help = "Guest ELF when program_elf_hex is empty")]
        program_elf: Option<PathBuf>,
        #[arg(long, help = "JSON file; otherwise read stdin")]
        arg_file: Option<PathBuf>,
    },
}

#[tokio::main]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("{err:#}");
        let out = SubmitResult::err(format!("{err:#}"));
        println!("{}", serde_json::to_string(&out).unwrap_or_default());
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::SubmitPublicTx {
            wallet_config,
            wallet_storage,
            program_elf,
            arg_file,
        } => {
            let json = read_json_payload(arg_file.as_deref())?;
            let payload = parse_payload_json(&json)?;
            let elf_path = resolve_program_elf_path(program_elf);
            match submit_public_tx(
                &wallet_config,
                &wallet_storage,
                &payload,
                elf_path.as_deref(),
            )
            .await
            {
                Ok(tx_hash) => {
                    let out = SubmitResult::ok(tx_hash);
                    println!("{}", serde_json::to_string(&out)?);
                }
                Err(err) => {
                    let out = SubmitResult::err(format!("{err:#}"));
                    println!("{}", serde_json::to_string(&out)?);
                    anyhow::bail!("{err:#}");
                }
            }
        }
    }
    Ok(())
}

fn read_json_payload(arg_file: Option<&std::path::Path>) -> Result<String> {
    if let Some(path) = arg_file {
        return std::fs::read_to_string(path)
            .with_context(|| format!("read arg file {}", path.display()));
    }
    let mut buf = String::new();
    io::stdin()
        .read_to_string(&mut buf)
        .context("read submit JSON from stdin")?;
    if buf.trim().is_empty() {
        anyhow::bail!("empty submit JSON (use --arg-file or stdin)");
    }
    Ok(buf)
}
