//! Localnet fixture helper (integration Step 10a).
//!
//! Submits initialize_vault, deposit, and create_stream using core [`Instruction`] encoding
//! (works around SPEL CLI `VaultId` IDL serialization). Writes `fixtures/localnet.json`.
//!
//! Uses LEZ 491 wallet + `lee` crates (same pin as `scaffold.toml`); requires `LEE_WALLET_HOME_DIR`.

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use base58::FromBase58;
use clap::{Parser, Subcommand};
use common::transaction::LeeTransaction;
use lee::program::Program as LeeProgram;
use lee::public_transaction::{Message, WitnessSet};
use lee::{AccountId as LeeAccountId, PublicTransaction};
use lee_core::account::Balance;
use lee_core::program::ProgramId as LeeProgramId;
use lez_payment_streams_core::{
    create_stream_instruction_accounts, deposit_instruction_accounts,
    derive_stream_config_account_id, derive_vault_account_ids,
    initialize_vault_instruction_accounts, Instruction, StreamId, TokensPerSecond, VaultId,
    VaultPrivacyTier, CLOCK_10_PROGRAM_ACCOUNT_ID,
};
use nssa::program::Program;
use nssa_core::account::AccountId as CoreAccountId;
use nssa_core::program::ProgramId as CoreProgramId;
use sequencer_service_rpc::RpcClient as _;
use serde::Serialize;
use wallet::WalletCore;

const DEFAULT_SEQUENCER: &str = "http://127.0.0.1:3040";
/// Local pinata topup is well below 500 on typical scaffold localnets; keep deposit + allocation within one claim.
const DEFAULT_DEPOSIT: Balance = 100;
/// Slow accrual so stream `0` stays non-depleted across repeated local demos (same terms as testnet table in step12).
const DEFAULT_STREAM_RATE: TokensPerSecond = 1;
const DEFAULT_STREAM_ALLOCATION: Balance = 80;

#[derive(Parser)]
#[command(name = "seed_localnet_fixture")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Derive PDAs and write fixture JSON (no chain I/O).
    WriteManifest {
        #[arg(long)]
        program_bin: PathBuf,
        #[arg(long)]
        owner: String,
        #[arg(long)]
        provider: String,
        #[arg(long, default_value = "0")]
        vault_id: u64,
        #[arg(long, default_value = "0")]
        stream_id: u64,
        #[arg(long, default_value = DEFAULT_SEQUENCER)]
        sequencer_url: String,
        #[arg(long, default_value = "fixtures/localnet.json")]
        output: PathBuf,
    },
    /// Submit demo vault lifecycle txs via scaffold wallet + sequencer.
    SeedOnchain {
        #[arg(long)]
        program_bin: PathBuf,
        #[arg(long)]
        owner: String,
        #[arg(long)]
        provider: String,
        #[arg(long, default_value = "0")]
        vault_id: u64,
        #[arg(long, default_value = "0")]
        stream_id: u64,
        #[arg(long, default_value_t = DEFAULT_DEPOSIT)]
        deposit_amount: Balance,
        #[arg(long, default_value_t = DEFAULT_STREAM_RATE)]
        stream_rate: TokensPerSecond,
        #[arg(long, default_value_t = DEFAULT_STREAM_ALLOCATION)]
        stream_allocation: Balance,
        #[arg(long, default_value_t = true)]
        skip_if_initialized: bool,
        #[arg(long)]
        force: bool,
        #[arg(long, default_value = DEFAULT_SEQUENCER)]
        sequencer_url: String,
        #[arg(long, default_value = "fixtures/localnet.json")]
        write_manifest: PathBuf,
    },
}

#[derive(Serialize)]
struct LocalnetFixture {
    schema_version: u32,
    sequencer_url: String,
    program_id_hex: String,
    owner_account_id: String,
    provider_account_id: String,
    vault_id: u64,
    stream_id: u64,
    vault_config_account_id: String,
    vault_holding_account_id: String,
    stream_config_account_id: String,
    clock_10_account_id: String,
    demo_deposit_amount: Balance,
    stream_rate: TokensPerSecond,
    stream_allocation: Balance,
    reserved_for_step_11b: String,
}

fn ensure_wallet_home_env() -> Result<()> {
    if std::env::var("LEE_WALLET_HOME_DIR").is_ok() {
        return Ok(());
    }
    if let Ok(legacy) = std::env::var("NSSA_WALLET_HOME_DIR") {
        std::env::set_var("LEE_WALLET_HOME_DIR", legacy);
        return Ok(());
    }
    Err(anyhow!(
        "set LEE_WALLET_HOME_DIR to scaffold wallet dir (see docs/step10a-local-chain-fixture.md)"
    ))
}

fn account_id_from_base58(raw: &str) -> Result<CoreAccountId> {
    let s = raw.strip_prefix("Public/").unwrap_or(raw);
    let bytes = s
        .from_base58()
        .map_err(|e| anyhow!("invalid base58 account id: {e:?}"))?;
    if bytes.len() != 32 {
        return Err(anyhow!("account id must be 32 bytes, got {}", bytes.len()));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(CoreAccountId::new(arr))
}

fn account_id_to_base58(id: CoreAccountId) -> String {
    id.to_string()
}

fn to_lee_account(id: CoreAccountId) -> LeeAccountId {
    LeeAccountId::new(id.into_value())
}

fn to_lee_program_id(id: CoreProgramId) -> LeeProgramId {
    id
}

fn program_id_from_bin(path: &PathBuf) -> Result<(CoreProgramId, String)> {
    let bytecode = std::fs::read(path)
        .with_context(|| format!("read program binary {}", path.display()))?;
    let program = Program::new(bytecode).context("parse guest Program")?;
    let pid = program.id();
    let hex: String = pid
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok((pid, hex))
}

fn build_fixture(
    sequencer_url: &str,
    program_id_hex: &str,
    program_id: &CoreProgramId,
    owner: CoreAccountId,
    provider: CoreAccountId,
    vault_id: VaultId,
    stream_id: StreamId,
    deposit_amount: Balance,
    stream_rate: TokensPerSecond,
    stream_allocation: Balance,
) -> LocalnetFixture {
    let (vault_config, vault_holding) = derive_vault_account_ids(program_id, owner, vault_id);
    let stream_config = derive_stream_config_account_id(program_id, vault_config, stream_id);
    LocalnetFixture {
        schema_version: 1,
        sequencer_url: sequencer_url.to_string(),
        program_id_hex: program_id_hex.to_string(),
        owner_account_id: account_id_to_base58(owner),
        provider_account_id: account_id_to_base58(provider),
        vault_id,
        stream_id,
        vault_config_account_id: account_id_to_base58(vault_config),
        vault_holding_account_id: account_id_to_base58(vault_holding),
        stream_config_account_id: account_id_to_base58(stream_config),
        clock_10_account_id: account_id_to_base58(CLOCK_10_PROGRAM_ACCOUNT_ID),
        demo_deposit_amount: deposit_amount,
        stream_rate,
        stream_allocation,
        reserved_for_step_11b:
            "Use a fresh vault_id (e.g. 1) or reset .scaffold/state/ for module-driven init tests."
                .to_string(),
    }
}

fn write_manifest(path: &PathBuf, fixture: &LocalnetFixture) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let json = serde_json::to_string_pretty(fixture)?;
    std::fs::write(path, json)?;
    eprintln!("Wrote {}", path.display());
    Ok(())
}

async fn submit_instruction(
    wallet: &WalletCore,
    program_id: LeeProgramId,
    account_ids: Vec<LeeAccountId>,
    instruction: Instruction,
    signer_ids: Vec<LeeAccountId>,
) -> Result<()> {
    let nonces = if signer_ids.is_empty() {
        vec![]
    } else {
        wallet
            .get_accounts_nonces(signer_ids.clone())
            .await
            .context("fetch signer nonces")?
    };

    let signing_keys: Vec<_> = signer_ids
        .iter()
        .map(|id| {
            wallet
                .get_account_public_signing_key(*id)
                .ok_or_else(|| anyhow!("signing key not found for {id}"))
        })
        .collect::<Result<Vec<_>>>()?;

    let message =
        Message::try_new(program_id, account_ids, nonces, instruction).context("build message")?;
    let witness_set = WitnessSet::for_message(&message, &signing_keys);
    let tx = PublicTransaction::new(message, witness_set);

    let tx_hash = wallet
        .sequencer_client
        .send_transaction(LeeTransaction::Public(tx))
        .await
        .context("submit transaction")?;
    eprintln!("Submitted tx {tx_hash}, waiting for confirmation…");

    let poller = wallet::poller::TxPoller::new(
        wallet.config(),
        wallet.sequencer_client.clone(),
    );
    poller
        .poll_tx(tx_hash)
        .await
        .context("confirm transaction")?;
    eprintln!("Confirmed {tx_hash}");
    Ok(())
}

async fn account_has_data(wallet: &WalletCore, account_id: LeeAccountId) -> Result<bool> {
    let acc = wallet
        .sequencer_client
        .get_account(account_id)
        .await
        .with_context(|| format!("getAccount {account_id}"))?;
    Ok(!acc.data.is_empty())
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::WriteManifest {
            program_bin,
            owner,
            provider,
            vault_id,
            stream_id,
            sequencer_url,
            output,
        } => {
            let (program_id, program_id_hex) = program_id_from_bin(&program_bin)?;
            let owner_id = account_id_from_base58(&owner)?;
            let provider_id = account_id_from_base58(&provider)?;
            let fixture = build_fixture(
                &sequencer_url,
                &program_id_hex,
                &program_id,
                owner_id,
                provider_id,
                vault_id,
                stream_id,
                DEFAULT_DEPOSIT,
                DEFAULT_STREAM_RATE,
                DEFAULT_STREAM_ALLOCATION,
            );
            write_manifest(&output, &fixture)?;
        },
        Commands::SeedOnchain {
            program_bin,
            owner,
            provider,
            vault_id,
            stream_id,
            deposit_amount,
            stream_rate,
            stream_allocation,
            skip_if_initialized,
            force,
            sequencer_url,
            write_manifest: manifest_path,
        } => {
            ensure_wallet_home_env()?;
            let skip_if_initialized = skip_if_initialized && !force;

            let (program_id, program_id_hex) = program_id_from_bin(&program_bin)?;
            let lee_program_id = to_lee_program_id(program_id);
            let owner_id = account_id_from_base58(&owner)?;
            let provider_id = account_id_from_base58(&provider)?;
            let owner_lee = to_lee_account(owner_id);

            let init_accounts =
                initialize_vault_instruction_accounts(&program_id, owner_id, vault_id);
            let vault_config_lee = to_lee_account(init_accounts[0]);

            let stream_accounts = create_stream_instruction_accounts(
                &program_id,
                owner_id,
                vault_id,
                stream_id,
                CLOCK_10_PROGRAM_ACCOUNT_ID,
            );
            let stream_config_lee = to_lee_account(stream_accounts[2]);

            let wallet =
                WalletCore::from_env().context("open wallet (491 storage + LEE_WALLET_HOME_DIR)")?;

            let vault_ready = account_has_data(&wallet, vault_config_lee).await?;
            let stream_ready = account_has_data(&wallet, stream_config_lee).await?;

            if skip_if_initialized && vault_ready && stream_ready {
                eprintln!(
                    "Vault {} and stream {} already initialized; skipping on-chain seed.",
                    account_id_to_base58(init_accounts[0]),
                    account_id_to_base58(stream_accounts[2]),
                );
            } else {
                let auth_transfer = LeeProgram::authenticated_transfer_program().id();

                if !vault_ready {
                    submit_instruction(
                        &wallet,
                        lee_program_id,
                        init_accounts.iter().copied().map(to_lee_account).collect(),
                        Instruction::initialize_vault(vault_id, VaultPrivacyTier::Public),
                        vec![owner_lee],
                    )
                    .await
                    .context("initialize_vault")?;
                } else {
                    eprintln!(
                        "Vault config {} already initialized; skipping initialize_vault.",
                        account_id_to_base58(init_accounts[0]),
                    );
                }

                if !stream_ready {
                    let deposit_accounts =
                        deposit_instruction_accounts(&program_id, owner_id, vault_id);
                    submit_instruction(
                        &wallet,
                        lee_program_id,
                        deposit_accounts.iter().copied().map(to_lee_account).collect(),
                        Instruction::Deposit {
                            vault_id,
                            amount: deposit_amount,
                            authenticated_transfer_program_id: auth_transfer,
                        },
                        vec![owner_lee],
                    )
                    .await
                    .context("deposit")?;

                    submit_instruction(
                        &wallet,
                        lee_program_id,
                        stream_accounts.iter().copied().map(to_lee_account).collect(),
                        Instruction::CreateStream {
                            vault_id,
                            stream_id,
                            provider: provider_id,
                            rate: stream_rate,
                            allocation: stream_allocation,
                        },
                        vec![owner_lee],
                    )
                    .await
                    .context("create_stream")?;
                } else {
                    eprintln!(
                        "Stream config {} already initialized; skipping deposit and create_stream.",
                        account_id_to_base58(stream_accounts[2]),
                    );
                }
            }

            let fixture = build_fixture(
                &sequencer_url,
                &program_id_hex,
                &program_id,
                owner_id,
                provider_id,
                vault_id,
                stream_id,
                deposit_amount,
                stream_rate,
                stream_allocation,
            );
            write_manifest(&manifest_path, &fixture)?;
        },
    }
    Ok(())
}
