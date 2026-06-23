//! Step 18 Part B — testnet vault/stream bootstrap via rc3 submit helper + 510 wallet reads.

use std::path::PathBuf;
use std::process::Command;

use anyhow::{anyhow, bail, Context, Result};
use base58::FromBase58;
use clap::Parser;
use lee::program::Program as LeeProgram;
use lee_core::account::Balance;
use lee_core::program::ProgramId as CoreProgramId;
use lez_payment_streams_core::{
    create_stream_instruction_accounts, deposit_instruction_accounts,
    derive_vault_account_ids,
    initialize_vault_instruction_accounts, instruction_bytes_for_public_transaction,
    Instruction, TokensPerSecond, VaultPrivacyTier,
    CLOCK_10_PROGRAM_ACCOUNT_ID,
};
use serde::Deserialize;
use serde::Serialize;

const DEFAULT_SEQUENCER: &str = "https://testnet.lez.logos.co/";

#[derive(Parser)]
#[command(name = "bootstrap_testnet_fixture")]
struct Args {
    #[arg(long)]
    program_bin: PathBuf,
    #[arg(long)]
    owner: String,
    #[arg(long)]
    provider: String,
    #[arg(long, default_value = "")]
    program_id_hex: String,
    #[arg(long)]
    rc3_wallet_config: PathBuf,
    #[arg(long)]
    rc3_wallet_storage: PathBuf,
    #[arg(long, env = "LEZ_TESTNET_SUBMIT", default_value = "lez-testnet-submit")]
    submit_helper: PathBuf,
    #[arg(long, env = "AUTH_TRANSFER_PROGRAM_HEX")]
    auth_transfer_program_hex: Option<String>,
    #[arg(long, default_value = "0")]
    vault_id: u64,
    #[arg(long, default_value = "0")]
    stream_id: u64,
    #[arg(long, default_value_t = 2000)]
    deposit_amount: Balance,
    #[arg(long, default_value_t = 1)]
    stream_rate: TokensPerSecond,
    #[arg(long, default_value_t = 1800)]
    stream_allocation: Balance,
    #[arg(long, default_value = DEFAULT_SEQUENCER)]
    sequencer_url: String,
    #[arg(long, default_value = "fixtures/testnet.json")]
    write_manifest: PathBuf,
    #[arg(long, default_value_t = true)]
    skip_if_initialized: bool,
    #[arg(long)]
    force: bool,
}

#[derive(Serialize)]
struct TestnetFixture {
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
    reserved_for_step_18: String,
}

#[derive(Deserialize)]
struct GetAccountOut {
    success: bool,
    has_data: bool,
}

fn account_id_from_base58_owner(raw: &str) -> Result<lee_core::account::AccountId> {
    let s = raw.strip_prefix("Public/").unwrap_or(raw);
    let bytes = s
        .from_base58()
        .map_err(|e| anyhow!("invalid base58 account id: {e:?}"))?;
    if bytes.len() != 32 {
        bail!("account id must be 32 bytes, got {}", bytes.len());
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(lee_core::account::AccountId::new(arr))
}

fn account_id_to_base58(id: lee_core::account::AccountId) -> String {
    id.to_string()
}

fn program_id_from_bin(path: &PathBuf) -> Result<(CoreProgramId, String)> {
    let bytecode = std::fs::read(path)
        .with_context(|| format!("read program binary {}", path.display()))?;
    let program = LeeProgram::new(bytecode).context("parse guest Program")?;
    let pid = program.id();
    let hex: String = pid
        .iter()
        .flat_map(|w| w.to_le_bytes())
        .map(|b| format!("{b:02x}"))
        .collect();
    Ok((pid, hex))
}

fn program_id_from_hex32(hex_str: &str) -> Result<CoreProgramId> {
    let bytes = hex::decode(hex_str.trim()).context("auth transfer program hex")?;
    if bytes.len() != 32 {
        bail!("program id hex must be 32 bytes");
    }
    let mut words = [0u32; 8];
    for (idx, chunk) in bytes.chunks_exact(4).enumerate() {
        words[idx] = u32::from_le_bytes(chunk.try_into().unwrap());
    }
    Ok(words)
}

fn resolve_auth_transfer_hex(args: &Args) -> Result<String> {
    if let Some(hex) = &args.auth_transfer_program_hex {
        return Ok(hex.trim().to_string());
    }
    let out = Command::new(&args.submit_helper)
        .arg("auth-transfer-program-id-hex")
        .output()
        .with_context(|| format!("spawn {}", args.submit_helper.display()))?;
    if !out.status.success() {
        bail!(
            "auth-transfer-program-id-hex failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn account_hex(id: lee_core::account::AccountId) -> String {
    hex::encode(id.as_ref())
}

fn build_submit_json(
    account_ids: &[lee_core::account::AccountId],
    instruction: &Instruction,
    owner: lee_core::account::AccountId,
) -> Result<String> {
    let instruction_bytes = instruction_bytes_for_public_transaction(instruction)
        .map_err(|e| anyhow!("instruction bytes: {e}"))?;
    let signing: Vec<bool> = account_ids
        .iter()
        .map(|id| id.as_ref() == owner.as_ref())
        .collect();
    let payload = serde_json::json!({
        "account_ids": account_ids.iter().map(|id| account_hex(*id)).collect::<Vec<_>>(),
        "signing_requirements": signing,
        "instruction_hex": hex::encode(instruction_bytes),
        "program_elf_hex": "",
        "program_dependencies_hex": [],
    });
    Ok(serde_json::to_string(&payload)?)
}

fn submit_via_helper(args: &Args, payload_json: &str) -> Result<()> {
    let mut child = Command::new(&args.submit_helper)
        .args([
            "submit-public-tx",
            "--wallet-config",
            args.rc3_wallet_config.to_str().unwrap(),
            "--wallet-storage",
            args.rc3_wallet_storage.to_str().unwrap(),
            "--program-elf",
            args.program_bin.to_str().unwrap(),
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .with_context(|| format!("spawn {}", args.submit_helper.display()))?;
    use std::io::Write;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(payload_json.as_bytes())?;
    }
    let out = child.wait_with_output()?;
    if !out.status.success() {
        bail!(
            "submit-public-tx failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }
    eprintln!("submit ok: {}", String::from_utf8_lossy(&out.stdout).trim());
    Ok(())
}

fn account_has_data_helper(args: &Args, account_id: lee_core::account::AccountId) -> Result<bool> {
    let out = Command::new(&args.submit_helper)
        .args([
            "get-account-public",
            "--wallet-config",
            args.rc3_wallet_config.to_str().unwrap(),
            "--wallet-storage",
            args.rc3_wallet_storage.to_str().unwrap(),
            "--account-id-hex",
            &account_hex(account_id),
        ])
        .output()
        .with_context(|| format!("get-account-public via {}", args.submit_helper.display()))?;
    if !out.status.success() {
        return Ok(false);
    }
    let parsed: GetAccountOut = serde_json::from_slice(&out.stdout).unwrap_or(GetAccountOut {
        success: false,
        has_data: false,
    });
    Ok(parsed.success && parsed.has_data)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let skip_if_initialized = args.skip_if_initialized && !args.force;

    let (program_id, program_id_hex_elf) = program_id_from_bin(&args.program_bin)?;
    if !args.program_id_hex.is_empty() && args.program_id_hex.trim() != program_id_hex_elf {
        bail!(
            "program_id_hex {} != ELF {}",
            args.program_id_hex,
            program_id_hex_elf
        );
    }
    let program_id_hex = if args.program_id_hex.is_empty() {
        program_id_hex_elf
    } else {
        args.program_id_hex.clone()
    };

    let owner_id = account_id_from_base58_owner(&args.owner)?;
    let provider_id = account_id_from_base58_owner(&args.provider)?;
    let vault_id = args.vault_id;
    let stream_id = args.stream_id;

    let auth_hex = resolve_auth_transfer_hex(&args)?;
    let auth_transfer = program_id_from_hex32(&auth_hex)?;

    let init_accounts =
        initialize_vault_instruction_accounts(&program_id, owner_id, vault_id);
    let (_, vault_holding) = derive_vault_account_ids(&program_id, owner_id, vault_id);
    let stream_accounts = create_stream_instruction_accounts(
        &program_id,
        owner_id,
        vault_id,
        stream_id,
        CLOCK_10_PROGRAM_ACCOUNT_ID,
    );

    let vault_ready = account_has_data_helper(&args, init_accounts[0])?;
    let holding_ready = account_has_data_helper(&args, vault_holding)?;
    let stream_ready = account_has_data_helper(&args, stream_accounts[2])?;

    if !vault_ready {
        let json = build_submit_json(
            &init_accounts,
            &Instruction::initialize_vault(vault_id, VaultPrivacyTier::Public),
            owner_id,
        )?;
        submit_via_helper(&args, &json)?;
    } else if skip_if_initialized {
        eprintln!("vault config exists; skip initialize_vault");
    }

    if !holding_ready || !skip_if_initialized {
        let deposit_accounts = deposit_instruction_accounts(&program_id, owner_id, vault_id);
        let json = build_submit_json(
            &deposit_accounts,
            &Instruction::Deposit {
                vault_id,
                amount: args.deposit_amount,
                authenticated_transfer_program_id: auth_transfer,
            },
            owner_id,
        )?;
        submit_via_helper(&args, &json)?;
    } else {
        eprintln!("vault holding funded; skip deposit");
    }

    if stream_ready && skip_if_initialized {
        eprintln!("stream exists; skip create_stream");
    } else if !stream_ready {
        let json = build_submit_json(
            &stream_accounts,
            &Instruction::CreateStream {
                vault_id,
                stream_id,
                provider: provider_id,
                rate: args.stream_rate,
                allocation: args.stream_allocation,
            },
            owner_id,
        )?;
        submit_via_helper(&args, &json)?;
    }

    let fixture = TestnetFixture {
        schema_version: 1,
        sequencer_url: args.sequencer_url.clone(),
        program_id_hex,
        owner_account_id: account_id_to_base58(owner_id),
        provider_account_id: account_id_to_base58(provider_id),
        vault_id,
        stream_id,
        vault_config_account_id: account_id_to_base58(init_accounts[0]),
        vault_holding_account_id: account_id_to_base58(vault_holding),
        stream_config_account_id: account_id_to_base58(stream_accounts[2]),
        clock_10_account_id: account_id_to_base58(CLOCK_10_PROGRAM_ACCOUNT_ID),
        demo_deposit_amount: args.deposit_amount,
        stream_rate: args.stream_rate,
        stream_allocation: args.stream_allocation,
        reserved_for_step_18: "bootstrap_testnet_fixture (Part B)".to_string(),
    };

    if let Some(parent) = args.write_manifest.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(
        &args.write_manifest,
        serde_json::to_string_pretty(&fixture)?,
    )?;
    eprintln!("Wrote {}", args.write_manifest.display());
    Ok(())
}
