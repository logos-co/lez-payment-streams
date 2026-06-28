//! Localnet fixture helper (integration Step 10a / Step 17b).
//!
//! Submits initialize_vault, deposit, and create_stream using core [`Instruction`] encoding
//! (works around SPEL CLI `VaultId` IDL serialization). Writes `fixtures/localnet.json`.
//!
//! Uses LEZ 491 wallet + `lee` crates (same pin as `scaffold.toml`); requires `LEE_WALLET_HOME_DIR`.

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{anyhow, bail, Context, Result};
use base58::FromBase58;
use borsh::BorshDeserialize;
use clap::{Parser, Subcommand};
use common::transaction::LeeTransaction;
use lee::program::Program as LeeProgram;
use lee::public_transaction::{Message, WitnessSet};
use lee::{AccountId as LeeAccountId, PublicTransaction};
use lee_core::account::Balance;
use lee_core::program::ProgramId as LeeProgramId;
use lez_payment_streams_core::{
    close_stream_instruction_accounts, create_stream_instruction_accounts, deposit_instruction_accounts,
    derive_stream_config_account_id, derive_vault_account_ids,
    initialize_vault_instruction_accounts, top_up_stream_instruction_accounts, ClockAccountData,
    Instruction, StreamId, TokensPerSecond, VaultConfig, VaultId, VaultPrivacyTier,
    CLOCK_10_PROGRAM_ACCOUNT_ID,
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
const DEFAULT_DEPOSIT: Balance = 1000;
const DEFAULT_STREAM_RATE: TokensPerSecond = 1;
const DEFAULT_ALLOCATION: Balance = 200;

#[derive(Parser)]
#[command(name = "seed_localnet_fixture")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Derive PDAs and write per-run fixture JSON (stream fields populated; no chain I/O).
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
        #[arg(long, default_value_t = DEFAULT_DEPOSIT)]
        deposit_amount: Balance,
        #[arg(long, default_value_t = DEFAULT_STREAM_RATE)]
        stream_rate: TokensPerSecond,
        #[arg(long, default_value_t = DEFAULT_ALLOCATION)]
        allocation: Balance,
        #[arg(long, default_value = DEFAULT_SEQUENCER)]
        sequencer_url: String,
        #[arg(long, default_value = "fixtures/localnet.json")]
        output: PathBuf,
    },
    /// Derive PDAs and write vault-only baseline manifest (schema v2; no stream fields).
    WriteVaultManifest {
        #[arg(long)]
        program_bin: PathBuf,
        #[arg(long)]
        owner: String,
        #[arg(long)]
        provider: String,
        #[arg(long, default_value = "0")]
        vault_id: u64,
        #[arg(long, default_value_t = DEFAULT_DEPOSIT)]
        deposit_amount: Balance,
        #[arg(long, default_value_t = DEFAULT_STREAM_RATE)]
        stream_rate: TokensPerSecond,
        #[arg(long, default_value_t = DEFAULT_ALLOCATION)]
        allocation: Balance,
        #[arg(long, default_value = DEFAULT_SEQUENCER)]
        sequencer_url: String,
        #[arg(long, default_value = "fixtures/localnet.json")]
        output: PathBuf,
    },
    /// Poll until on-chain Clock10 timestamp is within skew of wall time (post-restore gate).
    WaitClockSynced {
        #[arg(long, default_value_t = 5)]
        max_skew_s: u64,
        #[arg(long, default_value_t = 120)]
        timeout_s: u64,
        #[arg(long, default_value_t = 2)]
        poll_s: u64,
        #[arg(long, default_value = DEFAULT_SEQUENCER)]
        sequencer_url: String,
    },
    /// Print vault_config.next_stream_id from chain (stdout decimal).
    ReadVaultNextStreamId {
        #[arg(long)]
        program_bin: PathBuf,
        #[arg(long)]
        owner: String,
        #[arg(long, default_value = "0")]
        vault_id: u64,
        #[arg(long, default_value = DEFAULT_SEQUENCER)]
        sequencer_url: String,
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
    /// Deposit into an existing vault (ignores stream PDAs on chain).
    DepositOnchain {
        #[arg(long)]
        program_bin: PathBuf,
        #[arg(long)]
        owner: String,
        #[arg(long, default_value = "0")]
        vault_id: u64,
        #[arg(long)]
        deposit_amount: Balance,
        #[arg(long, default_value = DEFAULT_SEQUENCER)]
        sequencer_url: String,
    },
    /// Close stream on chain (owner wallet; provider is stream authority account).
    CloseStreamOnchain {
        #[arg(long)]
        program_bin: PathBuf,
        #[arg(long)]
        owner: String,
        #[arg(long)]
        provider: String,
        #[arg(long, default_value = "0")]
        vault_id: u64,
        #[arg(long)]
        stream_id: u64,
        #[arg(long, default_value = DEFAULT_SEQUENCER)]
        sequencer_url: String,
    },
    /// Increase stream allocation on chain (folds accrual; refreshes accrued_as_of).
    TopUpStreamOnchain {
        #[arg(long)]
        program_bin: PathBuf,
        #[arg(long)]
        owner: String,
        #[arg(long, default_value = "0")]
        vault_id: u64,
        #[arg(long)]
        stream_id: u64,
        #[arg(long)]
        increase_lo: Balance,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    stream_id: Option<u64>,
    vault_config_account_id: String,
    vault_holding_account_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream_config_account_id: Option<String>,
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

fn build_vault_baseline(
    sequencer_url: &str,
    program_id_hex: &str,
    program_id: &CoreProgramId,
    owner: CoreAccountId,
    provider: CoreAccountId,
    vault_id: VaultId,
    deposit_amount: Balance,
    stream_rate: TokensPerSecond,
    allocation: Balance,
) -> LocalnetFixture {
    let (vault_config, vault_holding) = derive_vault_account_ids(program_id, owner, vault_id);
    LocalnetFixture {
        schema_version: 2,
        sequencer_url: sequencer_url.to_string(),
        program_id_hex: program_id_hex.to_string(),
        owner_account_id: account_id_to_base58(owner),
        provider_account_id: account_id_to_base58(provider),
        vault_id,
        stream_id: None,
        vault_config_account_id: account_id_to_base58(vault_config),
        vault_holding_account_id: account_id_to_base58(vault_holding),
        stream_config_account_id: None,
        clock_10_account_id: account_id_to_base58(CLOCK_10_PROGRAM_ACCOUNT_ID),
        demo_deposit_amount: deposit_amount,
        stream_rate,
        allocation,
        reserved_for_step_11b:
            "Vault baseline only (Step 24c). Per-run stream_id written after create_stream-onchain."
                .to_string(),
    }
}

fn build_per_run_fixture(
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
        schema_version: 2,
        sequencer_url: sequencer_url.to_string(),
        program_id_hex: program_id_hex.to_string(),
        owner_account_id: account_id_to_base58(owner),
        provider_account_id: account_id_to_base58(provider),
        vault_id,
        stream_id: Some(stream_id),
        vault_config_account_id: account_id_to_base58(vault_config),
        vault_holding_account_id: account_id_to_base58(vault_holding),
        stream_config_account_id: Some(account_id_to_base58(stream_config)),
        clock_10_account_id: account_id_to_base58(CLOCK_10_PROGRAM_ACCOUNT_ID),
        demo_deposit_amount: deposit_amount,
        stream_rate,
        allocation,
        reserved_for_step_11b:
            "Per-run manifest after create_stream-onchain (Step 24c)."
                .to_string(),
    }
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
    build_per_run_fixture(
        sequencer_url,
        program_id_hex,
        program_id,
        owner,
        provider,
        vault_id,
        stream_id,
        deposit_amount,
        stream_rate,
        allocation,
    )
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

fn chain_timestamp_to_unix_seconds(ts: u64) -> u64 {
    if ts >= 1_000_000_000_000 {
        ts / 1000
    } else {
        ts
    }
}

fn wall_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

async fn read_clock10_timestamp(wallet: &WalletCore) -> Result<u64> {
    let clock_lee = to_lee_account(CLOCK_10_PROGRAM_ACCOUNT_ID);
    let acc = wallet
        .sequencer_client
        .get_account(clock_lee)
        .await
        .context("get Clock10 account")?;
    if acc.data.is_empty() {
        bail!("Clock10 account has no data");
    }
    let parsed = ClockAccountData::try_from_slice(&acc.data)
        .map_err(|e| anyhow!("decode ClockAccountData: {e}"))?;
    Ok(parsed.timestamp)
}

async fn wait_clock_synced(wallet: &WalletCore, max_skew_s: u64, timeout_s: u64, poll_s: u64) -> Result<()> {
    let deadline = wall_unix_seconds().saturating_add(timeout_s);
    loop {
        let clock_raw = read_clock10_timestamp(wallet).await?;
        let clock_s = chain_timestamp_to_unix_seconds(clock_raw);
        let wall_s = wall_unix_seconds();
        let skew = wall_s.saturating_sub(clock_s);
        eprintln!("wait-clock-synced: wall={wall_s} clock={clock_s} skew={skew}s (max {max_skew_s})");
        if skew <= max_skew_s {
            return Ok(());
        }
        if wall_s >= deadline {
            bail!(
                "Clock10 still {skew}s behind wall time after {timeout_s}s (clock={clock_s}, wall={wall_s})"
            );
        }
        tokio::time::sleep(Duration::from_secs(poll_s.max(1))).await;
    }
}

async fn vault_unallocated_lo(ctx: &OnchainContext) -> Result<Balance> {
    let init_accounts =
        initialize_vault_instruction_accounts(&ctx.program_id, ctx.owner_id, ctx.vault_id);
    let (_, vault_holding) =
        derive_vault_account_ids(&ctx.program_id, ctx.owner_id, ctx.vault_id);
    let vault_cfg_lee = to_lee_account(init_accounts[0]);
    let vault_holding_lee = to_lee_account(vault_holding);

    let cfg_acc = ctx
        .wallet
        .sequencer_client
        .get_account(vault_cfg_lee)
        .await
        .context("get vault config account")?;
    if cfg_acc.data.is_empty() {
        bail!("vault config account has no data");
    }
    let vault_cfg = VaultConfig::try_from_slice(&cfg_acc.data)
        .map_err(|e| anyhow!("decode VaultConfig: {e}"))?;

    let holding_acc = ctx
        .wallet
        .sequencer_client
        .get_account(vault_holding_lee)
        .await
        .context("get vault holding account")?;
    let holding_bal = holding_acc.balance;
    let total_allocated = vault_cfg.total_allocated;
    Ok(holding_bal.saturating_sub(total_allocated))
}

async fn vault_holding_balance_lo(ctx: &OnchainContext) -> Result<Balance> {
    let (_, vault_holding) =
        derive_vault_account_ids(&ctx.program_id, ctx.owner_id, ctx.vault_id);
    let holding_acc = ctx
        .wallet
        .sequencer_client
        .get_account(to_lee_account(vault_holding))
        .await
        .context("get vault holding account")?;
    Ok(holding_acc.balance)
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

    let unallocated = vault_unallocated_lo(ctx).await?;
    if allocation > unallocated {
        return Err(anyhow!(
            "vault unallocated {unallocated} < requested allocation {allocation}; run deposit-onchain"
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

async fn close_stream_onchain(
    ctx: &OnchainContext,
    provider: &str,
    stream_id: StreamId,
) -> Result<()> {
    let provider_id = account_id_from_base58(provider)?;
    let provider_lee = to_lee_account(provider_id);
    let accounts = close_stream_instruction_accounts(
        &ctx.program_id,
        ctx.owner_id,
        ctx.vault_id,
        stream_id,
        provider_id,
        CLOCK_10_PROGRAM_ACCOUNT_ID,
    );
    submit_instruction(
        &ctx.wallet,
        ctx.lee_program_id,
        accounts.iter().copied().map(to_lee_account).collect(),
        Instruction::CloseStream {
            vault_id: ctx.vault_id,
            stream_id,
        },
        vec![provider_lee],
    )
    .await
    .context("close_stream")?;
    Ok(())
}

async fn deposit_vault(ctx: &OnchainContext, deposit_amount: Balance) -> Result<()> {
    let init_accounts =
        initialize_vault_instruction_accounts(&ctx.program_id, ctx.owner_id, ctx.vault_id);
    let vault_config_lee = to_lee_account(init_accounts[0]);
    let vault_ready = account_has_data(&ctx.wallet, vault_config_lee).await?;
    if !vault_ready {
        return Err(anyhow!("vault must be initialized before deposit"));
    }
    let auth_transfer = LeeProgram::authenticated_transfer_program().id();
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
    Ok(())
}

async fn top_up_stream_onchain(
    ctx: &OnchainContext,
    stream_id: StreamId,
    increase_lo: Balance,
) -> Result<()> {
    if increase_lo == 0 {
        bail!("top-up increase must be non-zero");
    }
    let accounts = top_up_stream_instruction_accounts(
        &ctx.program_id,
        ctx.owner_id,
        ctx.vault_id,
        stream_id,
        CLOCK_10_PROGRAM_ACCOUNT_ID,
    );
    submit_instruction(
        &ctx.wallet,
        ctx.lee_program_id,
        accounts.iter().copied().map(to_lee_account).collect(),
        Instruction::TopUpStream {
            vault_id: ctx.vault_id,
            stream_id,
            vault_total_allocated_increase: increase_lo,
        },
        vec![ctx.owner_lee],
    )
    .await
    .context("top_up_stream")?;
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
            deposit_amount,
            stream_rate,
            allocation,
            sequencer_url,
            output,
        } => {
            let (program_id, program_id_hex) = program_id_from_bin(&program_bin)?;
            let owner_id = account_id_from_base58(&owner)?;
            let provider_id = account_id_from_base58(&provider)?;
            let fixture = build_per_run_fixture(
                &sequencer_url,
                &program_id_hex,
                &program_id,
                owner_id,
                provider_id,
                vault_id,
                stream_id,
                deposit_amount,
                stream_rate,
                allocation,
            );
            write_manifest(&output, &fixture)?;
        },
        Commands::WriteVaultManifest {
            program_bin,
            owner,
            provider,
            vault_id,
            deposit_amount,
            stream_rate,
            allocation,
            sequencer_url,
            output,
        } => {
            let (program_id, program_id_hex) = program_id_from_bin(&program_bin)?;
            let owner_id = account_id_from_base58(&owner)?;
            let provider_id = account_id_from_base58(&provider)?;
            let fixture = build_vault_baseline(
                &sequencer_url,
                &program_id_hex,
                &program_id,
                owner_id,
                provider_id,
                vault_id,
                deposit_amount,
                stream_rate,
                allocation,
            );
            write_manifest(&output, &fixture)?;
        },
        Commands::ReadVaultNextStreamId {
            program_bin,
            owner,
            vault_id,
            sequencer_url: _,
        } => {
            ensure_wallet_home_env()?;
            let ctx = open_onchain(&program_bin, &owner, vault_id).await?;
            let init_accounts =
                initialize_vault_instruction_accounts(&ctx.program_id, ctx.owner_id, ctx.vault_id);
            let vault_cfg_lee = to_lee_account(init_accounts[0]);
            let acc = ctx
                .wallet
                .sequencer_client
                .get_account(vault_cfg_lee)
                .await
                .context("get vault config account")?;
            if acc.data.is_empty() {
                bail!("vault config account has no data");
            }
            let cfg = VaultConfig::try_from_slice(&acc.data)
                .map_err(|e| anyhow!("decode VaultConfig: {e}"))?;
            println!("{}", cfg.next_stream_id);
        },
        Commands::WaitClockSynced {
            max_skew_s,
            timeout_s,
            poll_s,
            sequencer_url: _,
        } => {
            ensure_wallet_home_env()?;
            let wallet = WalletCore::from_env().context("open wallet for clock poll")?;
            wait_clock_synced(&wallet, max_skew_s, timeout_s, poll_s).await?;
            eprintln!("Clock10 synced to wall time (skew <= {max_skew_s}s)");
        },
        Commands::DepositOnchain {
            program_bin,
            owner,
            vault_id,
            deposit_amount,
            sequencer_url: _,
        } => {
            ensure_wallet_home_env()?;
            let ctx = open_onchain(&program_bin, &owner, vault_id).await?;
            deposit_vault(&ctx, deposit_amount).await?;
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
        Commands::CloseStreamOnchain {
            program_bin,
            owner,
            provider,
            vault_id,
            stream_id,
            sequencer_url: _,
        } => {
            ensure_wallet_home_env()?;
            let ctx = open_onchain(&program_bin, &owner, vault_id).await?;
            close_stream_onchain(&ctx, &provider, stream_id).await?;
        },
        Commands::TopUpStreamOnchain {
            program_bin,
            owner,
            vault_id,
            stream_id,
            increase_lo,
            sequencer_url: _,
        } => {
            ensure_wallet_home_env()?;
            let ctx = open_onchain(&program_bin, &owner, vault_id).await?;
            top_up_stream_onchain(&ctx, stream_id, increase_lo).await?;
        },
        Commands::CreateStreamOnchain {
            program_bin,
            owner,
            provider,
            vault_id,
            stream_id,
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
            let demo_deposit_amount = vault_holding_balance_lo(&ctx).await?;
            let fixture = build_fixture(
                &sequencer_url,
                &ctx.program_id_hex,
                &ctx.program_id,
                ctx.owner_id,
                provider_id,
                vault_id,
                stream_id,
                demo_deposit_amount,
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
            let demo_deposit_amount = vault_holding_balance_lo(&ctx).await?;
            let fixture = build_fixture(
                &sequencer_url,
                &ctx.program_id_hex,
                &ctx.program_id,
                ctx.owner_id,
                provider_id,
                vault_id,
                stream_id,
                demo_deposit_amount,
                stream_rate,
                allocation,
            );
            write_manifest(&manifest_path, &fixture)?;
        },
    }
    Ok(())
}
