use std::path::{Path, PathBuf};

use anyhow::{bail, Context as _, Result};
use base58::FromBase58;
use nssa::{program::Program, AccountId};
use serde::Deserialize;
use wallet::WalletCore;

use crate::legacy_sequencer::{LegacySequencerClient, submit_public_with_wallet};

#[derive(Debug, Deserialize)]
pub struct SubmitPayload {
    pub account_ids: Vec<String>,
    pub signing_requirements: Vec<bool>,
    pub instruction_hex: String,
    pub program_elf_hex: String,
    #[serde(default)]
    pub program_dependencies_hex: Vec<String>,
}

#[derive(Debug, serde::Serialize)]
pub struct SubmitResult {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl SubmitResult {
    pub fn ok(tx_hash: String) -> Self {
        Self {
            success: true,
            tx_hash: Some(tx_hash),
            error: None,
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            tx_hash: None,
            error: Some(message.into()),
        }
    }
}

pub fn account_id_hex_from_base58(raw: &str) -> Result<String> {
    let bytes: Vec<u8> = raw
        .from_base58()
        .map_err(|e| anyhow::anyhow!("base58 decode: {e:?}"))?;
    if bytes.len() != 32 {
        bail!("base58 account id must decode to 32 bytes, got {}", bytes.len());
    }
    Ok(hex::encode(bytes))
}

pub fn parse_account_id_hex(hex_str: &str) -> Result<AccountId> {
    let trimmed = hex_str.trim();
    if trimmed.len() != 64 {
        bail!("account id must be 64 hex chars, got {}", trimmed.len());
    }
    let bytes = hex::decode(trimmed).context("account id hex decode")?;
    let arr: [u8; 32] = bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("account id must be 32 bytes"))?;
    Ok(AccountId::new(arr))
}

pub fn parse_payload_json(json: &str) -> Result<SubmitPayload> {
    serde_json::from_str(json).context("submit payload JSON")
}

fn load_program_elf(payload: &SubmitPayload, program_elf_path: Option<&Path>) -> Result<Vec<u8>> {
    if !payload.program_elf_hex.trim().is_empty() {
        return hex::decode(payload.program_elf_hex.trim()).context("program_elf_hex decode");
    }
    let path = program_elf_path.context(
        "program_elf_hex empty: pass --program-elf or set PAYMENT_STREAMS_GUEST_BIN for the helper",
    )?;
    std::fs::read(path).with_context(|| format!("read program elf {}", path.display()))
}

pub async fn submit_public_tx(
    wallet_config: &Path,
    wallet_storage: &Path,
    payload: &SubmitPayload,
    program_elf_path: Option<&Path>,
) -> Result<String> {
    if payload.account_ids.len() != payload.signing_requirements.len() {
        bail!("account_ids and signing_requirements length mismatch");
    }
    if payload.instruction_hex.trim().is_empty() {
        bail!("instruction_hex empty");
    }

    let _deps_present = !payload.program_dependencies_hex.is_empty();

    let elf = load_program_elf(payload, program_elf_path)?;
    let program = Program::new(elf).context("invalid guest program ELF")?;
    let program_id = program.id();

    let account_ids: Vec<AccountId> = payload
        .account_ids
        .iter()
        .map(|h| parse_account_id_hex(h))
        .collect::<Result<_>>()?;

    let instruction_bytes = hex::decode(payload.instruction_hex.trim()).context("instruction_hex")?;
    let instruction_words = instruction_bytes_to_risc0_words(&instruction_bytes)?;

    let wallet = WalletCore::new_update_chain(
        wallet_config.to_path_buf(),
        wallet_storage.to_path_buf(),
        None,
    )
    .context("open wallet")?;

    let legacy = LegacySequencerClient::from_wallet_config(wallet_config)?;
    submit_public_with_wallet(
        &legacy,
        &wallet,
        payload,
        program_id,
        account_ids,
        instruction_words,
    )
    .await
}

pub fn instruction_bytes_to_risc0_words(bytes: &[u8]) -> Result<Vec<u32>> {
    let with_prefix = risc0_zkvm::serde::to_vec(bytes).context("risc0 serialize instruction")?;
    Ok(with_prefix[1..].to_vec())
}

pub fn resolve_program_elf_path(explicit: Option<PathBuf>) -> Option<PathBuf> {
    if let Some(p) = explicit {
        return Some(p);
    }
    std::env::var("PAYMENT_STREAMS_GUEST_BIN")
        .ok()
        .map(PathBuf::from)
        .filter(|p| !p.as_os_str().is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_payload_shape() {
        let json = r#"{
            "account_ids": ["0000000000000000000000000000000000000000000000000000000000000001"],
            "signing_requirements": [true],
            "instruction_hex": "0102",
            "program_elf_hex": "",
            "program_dependencies_hex": []
        }"#;
        let p = parse_payload_json(json).unwrap();
        assert_eq!(p.account_ids.len(), 1);
        assert!(p.signing_requirements[0]);
    }

    #[test]
    fn account_id_hex_must_be_32_bytes() {
        assert!(parse_account_id_hex("abcd").is_err());
        assert!(parse_account_id_hex(&"aa".repeat(32)).is_ok());
    }
}
