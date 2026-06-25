mod legacy_sequencer;
mod submit;

use std::{
    io::{self, Read as _},
    path::PathBuf,
};

use anyhow::{Context as _, Result};
use clap::{Parser, Subcommand};
use legacy_sequencer::LegacySequencerClient;
use nssa::program::Program as NssaProgram;
use submit::{
    account_id_hex_from_base58, parse_account_id_hex, parse_payload_json, resolve_program_elf_path,
    submit_public_tx, SubmitResult,
};
use wallet::WalletCore;

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
    /// Read on-chain account data (existence / bootstrap idempotency).
    GetAccountPublic {
        #[arg(long)]
        wallet_config: PathBuf,
        #[arg(long)]
        wallet_storage: PathBuf,
        #[arg(long, help = "64-char hex account id")]
        account_id_hex: String,
    },
    /// Print rc3 authenticated-transfer ProgramId (64 hex chars) for testnet deposit instructions.
    AuthTransferProgramIdHex,
    /// Deploy guest ELF via legacy send_tx (public testnet sequencer API).
    DeployProgram {
        #[arg(long)]
        wallet_config: PathBuf,
        #[arg(long, help = "Guest .bin path")]
        program_bin: PathBuf,
    },
    /// Print 64-char hex account id for a base58 LEZ account id string.
    AccountIdFromBase58 {
        account_base58: String,
    },
    /// Print built-in CLOCK_10 account id (base58) for manifest / read smoke.
    Clock10AccountBase58,
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
        Commands::GetAccountPublic {
            wallet_config,
            wallet_storage,
            account_id_hex,
        } => {
            let id = parse_account_id_hex(&account_id_hex)?;
            let _wallet = WalletCore::new_update_chain(
                wallet_config.clone(),
                wallet_storage,
                None,
            )
            .context("open wallet")?;
            let legacy = LegacySequencerClient::from_wallet_config(&wallet_config)?;
            let acc = legacy
                .get_account(id)
                .await
                .map_err(|e| anyhow::anyhow!("get_account: {e}"))?;
            let out = serde_json::json!({
                "success": true,
                "has_data": !acc.data.is_empty(),
                "balance": acc.balance,
                "program_owner_nonzero": acc.program_owner.as_ref().iter().any(|&w| w != 0),
            });
            println!("{}", serde_json::to_string(&out)?);
        }
        Commands::DeployProgram {
            wallet_config,
            program_bin,
        } => {
            let bytecode = std::fs::read(&program_bin)
                .with_context(|| format!("read {}", program_bin.display()))?;
            let program = NssaProgram::new(bytecode.clone()).context("parse guest ELF")?;
            let expected_id = program.id();
            let legacy = LegacySequencerClient::from_wallet_config(&wallet_config)?;
            match legacy.deploy_program_bytecode(bytecode).await {
                Ok(tx_hash) => {
                    let out = SubmitResult::ok(tx_hash);
                    println!("{}", serde_json::to_string(&out)?);
                    let hex: String = expected_id
                        .as_ref()
                        .iter()
                        .flat_map(|w| w.to_le_bytes())
                        .map(|b| format!("{b:02x}"))
                        .collect();
                    eprintln!("program_id_hex={hex}");
                }
                Err(err) => {
                    let msg = format!("{err:#}");
                    if msg.to_ascii_lowercase().contains("already")
                        || msg.to_ascii_lowercase().contains("exist")
                    {
                        let out = SubmitResult::ok(String::new());
                        println!("{}", serde_json::to_string(&out)?);
                        let hex: String = expected_id
                            .as_ref()
                            .iter()
                            .flat_map(|w| w.to_le_bytes())
                            .map(|b| format!("{b:02x}"))
                            .collect();
                        eprintln!("program_id_hex={hex} (already deployed)");
                    } else {
                        anyhow::bail!("{msg}");
                    }
                }
            }
        }
        Commands::AuthTransferProgramIdHex => {
            let id = NssaProgram::authenticated_transfer_program().id();
            let hex: String = id
                .as_ref()
                .iter()
                .flat_map(|w| w.to_le_bytes())
                .map(|b| format!("{b:02x}"))
                .collect();
            println!("{hex}");
        }
        Commands::AccountIdFromBase58 { account_base58 } => {
            let hex = account_id_hex_from_base58(&account_base58)?;
            println!("{hex}");
        }
        Commands::Clock10AccountBase58 => {
            use nssa::CLOCK_10_PROGRAM_ACCOUNT_ID;
            println!("{CLOCK_10_PROGRAM_ACCOUNT_ID}");
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
