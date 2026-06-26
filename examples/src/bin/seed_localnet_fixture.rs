//! Localnet fixture helper (integration Step 10a / Step 17b).
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
use lee::program::Program;
use lee_core::account::AccountId as CoreAccountId;
use lee_core::program::ProgramId as CoreProgramId;
use sequencer_service_rpc::RpcClient as _;
use serde::Serialize;
use wallet::WalletCore;

const DEFAULT_SEQUENCER: &str = "http://127.0.0.1:3040";
/// Local pinata topup is ~150 tokens per claim on typical scaffold localnets. Demo scripts
/// pass explicit deposit/topup counts (see `scripts/seed-localnet-fixture.sh`).
const DEFAULT_DEPOSIT: Balance = 450;
const DEFAULT_STREAM_RATE: TokensPerSecond = 1;
const DEFAULT_ALLOCATION: Balance = 400;

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
    /// Initialize vault and deposit only (Step 17b baseline snapshot; no stream).
    PrefundOnchain {
        #[arg(long)]
        program_bin: PathBuf,
        #[arg(long)]
        owner: String,
        #[arg(long, default_value = "0")]
        vault_id: u64,
        #[arg(long, default_value_t = DEFAULT_DEPOSIT)]
        deposit_amount: Balance,
        #[arg(long, default_value_t = true)]
        skip_if_initialized: bool,
        #[arg(long)]
        force: bool,
        #[arg(long, default_value = DEFAULT_SEQUENCER)]
        sequencer_url: String,
    },
    /// Create stream on a funded vault and write fixture JSON (Step 17b per-run).
    CreateStreamOnchain {
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
        #[arg(long, default_value_t = DEFAULT_ALLOCATION)]
        allocation: Balance,
        #[arg(long, default_value_t = true)]
        skip_if_initialized: bool,
        #[arg(long)]
        force: bool,
        #[arg(long, default_value = DEFAULT_SEQUENCER)]
        sequencer_url: String,
        #[arg(long, default_value = "fixtures/localnet.json")]
        write_manifest: PathBuf,
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
        #[arg(long, default_value_t = DEFAULT_ALLOCATION)]
        allocation: Balance,
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
    allocation: Balance,
    reserved_for_step_11b: String,
}

struct OnchainContext {
    program_id: CoreProgramId,
    program_id_hex: String,
    lee_program_id: LeeProgramId,
    owner_id: CoreAccountId,
    owner_lee: LeeAccountId,
    vault_id: VaultId,
    wallet: WalletCore,
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
    allocation: Balance,
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
        allocation,
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

async fn open_onchain(
    program_bin: &PathBuf,
    owner: &str,
    vault_id: VaultId,
) -> Result<OnchainContext> {
    let (program_id, program_id_hex) = program_id_from_bin(program_bin)?;
    let owner_id = account_id_from_base58(owner)?;
    let wallet = WalletCore::from_env().context("open wallet (491 storage + LEE_WALLET_HOME_DIR)")?;
    Ok(OnchainContext {
        lee_program_id: to_lee_program_id(program_id),
        program_id,
        program_id_hex,
        owner_id,
        owner_lee: to_lee_account(owner_id),
        vault_id,
        wallet,
    })
}

async fn prefund_vault(
    ctx: &OnchainContext,
    deposit_amount: Balance,
    skip_if_initialized: bool,
) -> Result<()> {
    let init_accounts =
        initialize_vault_instruction_accounts(&ctx.program_id, ctx.owner_id, ctx.vault_id);
    let vault_config_lee = to_lee_account(init_accounts[0]);
    let (_, vault_holding) =
        derive_vault_account_ids(&ctx.program_id, ctx.owner_id, ctx.vault_id);
    let vault_holding_lee = to_lee_account(vault_holding);

    let stream_accounts = create_stream_instruction_accounts(
        &ctx.program_id,
        ctx.owner_id,
        ctx.vault_id,
        0,
        CLOCK_10_PROGRAM_ACCOUNT_ID,
    );
    let stream_config_lee = to_lee_account(stream_accounts[2]);

    let vault_ready = account_has_data(&ctx.wallet, vault_config_lee).await?;
    let holding_ready = account_has_data(&ctx.wallet, vault_holding_lee).await?;
    let stream_ready = account_has_data(&ctx.wallet, stream_config_lee).await?;

    if stream_ready {
        return Err(anyhow!(
            "stream config already exists on chain; prefund baseline must be pre-stream (reset or restore snapshot)"
        ));
    }

    if skip_if_initialized && vault_ready && holding_ready {
        eprintln!(
            "Vault {} funded baseline already present; skipping prefund.",
            account_id_to_base58(init_accounts[0]),
        );
        return Ok(());
    }

    let auth_transfer = LeeProgram::authenticated_transfer_program().id();

    if !vault_ready {
        submit_instruction(
            &ctx.wallet,
            ctx.lee_program_id,
            init_accounts.iter().copied().map(to_lee_account).collect(),
            Instruction::initialize_vault(ctx.vault_id, VaultPrivacyTier::Public),
            vec![ctx.owner_lee],
        )
        .await
        .context("initialize_vault")?;
    } else {
        eprintln!(
            "Vault config {} already initialized; skipping initialize_vault.",
            account_id_to_base58(init_accounts[0]),
        );
    }

    if !holding_ready || !skip_if_initialized {
        let deposit_accounts =
            deposit_instruction_accounts(&ctx.program_id, ctx.owner_id, ctx.vault_id);
        submit_instruction(
            &ctx.wallet,
            ctx.lee_program_id,
            deposit_accounts.iter().copied().map(to_lee_account).collect(),
            Instruction::Deposit {
                vault_id: ctx.vault_id,
                amount: deposit_amount,
                authenticated_transfer_program_id: auth_transfer,
            },
            vec![ctx.owner_lee],
        )
        .await
        .context("deposit")?;
    } else {
        eprintln!("Vault holding already funded; skipping deposit.");
    }

    Ok(())
}

async fn create_stream_onchain(
    ctx: &OnchainContext,
    provider: &str,
    stream_id: StreamId,
    stream_rate: TokensPerSecond,
    allocation: Balance,
    skip_if_initialized: bool,
) -> Result<()> {
    let provider_id = account_id_from_base58(provider)?;

    let init_accounts =
        initialize_vault_instruction_accounts(&ctx.program_id, ctx.owner_id, ctx.vault_id);
    let vault_config_lee = to_lee_account(init_accounts[0]);
    let (_, vault_holding) =
        derive_vault_account_ids(&ctx.program_id, ctx.owner_id, ctx.vault_id);
    let vault_holding_lee = to_lee_account(vault_holding);

    let stream_accounts = create_stream_instruction_accounts(
        &ctx.program_id,
        ctx.owner_id,
        ctx.vault_id,
        stream_id,
        CLOCK_10_PROGRAM_ACCOUNT_ID,
    );
    let stream_config_lee = to_lee_account(stream_accounts[2]);

    let vault_ready = account_has_data(&ctx.wallet, vault_config_lee).await?;
    let holding_ready = account_has_data(&ctx.wallet, vault_holding_lee).await?;
    let stream_ready = account_has_data(&ctx.wallet, stream_config_lee).await?;

    if !vault_ready || !holding_ready {
        return Err(anyhow!(
            "vault must be initialized and funded before create_stream (run prefund-onchain or restore snapshot)"
        ));
    }

    if skip_if_initialized && stream_ready {
        eprintln!(
            "Stream config {} already initialized; skipping create_stream.",
            account_id_to_base58(stream_accounts[2]),
        );
        return Ok(());
    }

    if stream_ready {
        return Err(anyhow!(
            "stream config {} already exists (use fresh baseline restore)",
            account_id_to_base58(stream_accounts[2]),
        ));
    }

    submit_instruction(
        &ctx.wallet,
        ctx.lee_program_id,
        stream_accounts.iter().copied().map(to_lee_account).collect(),
        Instruction::CreateStream {
            vault_id: ctx.vault_id,
            stream_id,
            provider: provider_id,
            rate: stream_rate,
            allocation: allocation,
        },
        vec![ctx.owner_lee],
    )
    .await
    .context("create_stream")?;

    Ok(())
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
                DEFAULT_ALLOCATION,
            );
            write_manifest(&output, &fixture)?;
        },
        Commands::PrefundOnchain {
            program_bin,
            owner,
            vault_id,
            deposit_amount,
            skip_if_initialized,
            force,
            sequencer_url: _,
        } => {
            ensure_wallet_home_env()?;
            let skip_if_initialized = skip_if_initialized && !force;
            let ctx = open_onchain(&program_bin, &owner, vault_id).await?;
            prefund_vault(&ctx, deposit_amount, skip_if_initialized).await?;
        },
        Commands::CreateStreamOnchain {
            program_bin,
            owner,
            provider,
            vault_id,
            stream_id,
            deposit_amount,
            stream_rate,
            allocation,
            skip_if_initialized,
            force,
            sequencer_url,
            write_manifest: manifest_path,
        } => {
            ensure_wallet_home_env()?;
            let skip_if_initialized = skip_if_initialized && !force;
            let ctx = open_onchain(&program_bin, &owner, vault_id).await?;
            create_stream_onchain(
                &ctx,
                &provider,
                stream_id,
                stream_rate,
                allocation,
                skip_if_initialized,
            )
            .await?;
            let provider_id = account_id_from_base58(&provider)?;
            let fixture = build_fixture(
                &sequencer_url,
                &ctx.program_id_hex,
                &ctx.program_id,
                ctx.owner_id,
                provider_id,
                vault_id,
                stream_id,
                deposit_amount,
                stream_rate,
                allocation,
            );
            write_manifest(&manifest_path, &fixture)?;
        },
        Commands::SeedOnchain {
            program_bin,
            owner,
            provider,
            vault_id,
            stream_id,
            deposit_amount,
            stream_rate,
            allocation,
            skip_if_initialized,
            force,
            sequencer_url,
            write_manifest: manifest_path,
        } => {
            ensure_wallet_home_env()?;
            let skip_if_initialized = skip_if_initialized && !force;
            let ctx = open_onchain(&program_bin, &owner, vault_id).await?;
            prefund_vault(&ctx, deposit_amount, skip_if_initialized).await?;
            create_stream_onchain(
                &ctx,
                &provider,
                stream_id,
                stream_rate,
                allocation,
                skip_if_initialized,
            )
            .await?;
            let provider_id = account_id_from_base58(&provider)?;
            let fixture = build_fixture(
                &sequencer_url,
                &ctx.program_id_hex,
                &ctx.program_id,
                ctx.owner_id,
                provider_id,
                vault_id,
                stream_id,
                deposit_amount,
                stream_rate,
                allocation,
            );
            write_manifest(&manifest_path, &fixture)?;
        },
    }
    Ok(())
}
