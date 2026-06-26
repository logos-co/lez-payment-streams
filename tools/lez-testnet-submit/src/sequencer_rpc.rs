use std::path::Path;

use anyhow::{Context as _, Result};
use common::{transaction::NSSATransaction, HashType};
use nssa::{
    program_deployment_transaction::{Message as DeployMessage, ProgramDeploymentTransaction},
    public_transaction::{Message, WitnessSet},
    Account, AccountId, PrivateKey, ProgramId, PublicKey, PublicTransaction, Signature,
};
use sha2::{Digest as _, Sha256};
use sequencer_service_rpc::{RpcClient as _, SequencerClient, SequencerClientBuilder};
use serde_json::Value;
use url::Url;
use wallet::WalletCore;

#[derive(Clone)]
pub struct SequencerRpc {
    client: SequencerClient,
}

impl SequencerRpc {
    pub fn from_wallet_config(wallet_config: &Path) -> Result<Self> {
        let cfg: Value =
            serde_json::from_str(&std::fs::read_to_string(wallet_config).context("read wallet config")?)?;
        let addr = cfg
            .get("sequencer_addr")
            .and_then(|v| v.as_str())
            .context("wallet_config missing sequencer_addr")?;
        let url = Url::parse(addr).context("parse sequencer_addr")?;
        let client = SequencerClientBuilder::default()
            .build(url.as_str())
            .context("build jsonrpsee sequencer client")?;
        Ok(Self { client })
    }

    pub async fn last_block(&self) -> Result<u64> {
        self.client
            .get_last_block_id()
            .await
            .context("getLastBlockId")
    }

    pub async fn get_accounts_nonces(&self, account_ids: Vec<AccountId>) -> Result<Vec<u128>> {
        let nonces = self
            .client
            .get_accounts_nonces(account_ids)
            .await
            .context("getAccountsNonces")?;
        Ok(nonces.into_iter().map(u128::from).collect())
    }

    pub async fn get_account(&self, account_id: AccountId) -> Result<Account> {
        self.client
            .get_account(account_id)
            .await
            .context("getAccount")
    }

    pub async fn send_nssa_tx(&self, tx: NSSATransaction) -> Result<String> {
        let hash: HashType = self
            .client
            .send_transaction(tx)
            .await
            .context("sendTransaction")?;
        Ok(format!("{hash}"))
    }

    pub async fn send_public_tx(&self, tx: PublicTransaction) -> Result<String> {
        self.send_nssa_tx(NSSATransaction::Public(tx)).await
    }

    pub async fn deploy_program_bytecode(&self, bytecode: Vec<u8>) -> Result<String> {
        let message = DeployMessage::new(bytecode);
        let transaction = ProgramDeploymentTransaction::new(message);
        self.send_nssa_tx(NSSATransaction::ProgramDeployment(transaction))
            .await
    }
}

const PUBLIC_MESSAGE_HASH_PREFIX: &[u8; 32] = b"/LEE/v0.3/Message/Public/\x00\x00\x00\x00\x00\x00\x00";

fn public_message_signing_hash(message: &Message) -> [u8; 32] {
    let body = borsh::to_vec(message).expect("borsh Message");
    let mut bytes = Vec::with_capacity(PUBLIC_MESSAGE_HASH_PREFIX.len() + body.len());
    bytes.extend_from_slice(PUBLIC_MESSAGE_HASH_PREFIX);
    bytes.extend_from_slice(&body);
    Sha256::digest(bytes).into()
}

fn witness_set_for_testnet_message(
    message: &Message,
    private_keys: &[&PrivateKey],
) -> WitnessSet {
    let digest = public_message_signing_hash(message);
    let signatures_and_public_keys = private_keys
        .iter()
        .map(|&key| {
            (
                Signature::new(key, &digest),
                PublicKey::new_from_private_key(key),
            )
        })
        .collect();
    WitnessSet::from_raw_parts(signatures_and_public_keys)
}

fn witness_valid_for_testnet_message(message: &Message, witness_set: &WitnessSet) -> bool {
    let digest = public_message_signing_hash(message);
    witness_set
        .signatures_and_public_keys()
        .iter()
        .all(|(signature, public_key)| signature.is_valid_for(&digest, public_key))
}

pub async fn submit_public_with_wallet(
    wallet: &WalletCore,
    payload: &super::submit::SubmitPayload,
    program_id: ProgramId,
    account_ids: Vec<AccountId>,
    instruction_words: Vec<u32>,
) -> Result<String> {
    let mut signing_account_ids = Vec::new();
    let mut private_keys = Vec::new();
    for (account_id, needs_sign) in account_ids.iter().zip(payload.signing_requirements.iter()) {
        if *needs_sign {
            signing_account_ids.push(*account_id);
            let key = wallet
                .storage()
                .user_data
                .get_pub_account_signing_key(*account_id)
                .ok_or_else(|| anyhow::anyhow!("signing key not found for {account_id:?}"))?;
            private_keys.push(key);
        }
    }

    let nonces: Vec<nssa_core::account::Nonce> = wallet
        .get_accounts_nonces(signing_account_ids)
        .await
        .context("get_accounts_nonces")?
        .into_iter()
        .map(Into::into)
        .collect();

    let message = Message::new_preserialized(program_id, account_ids, nonces, instruction_words);
    let key_refs: Vec<&PrivateKey> = private_keys.iter().copied().collect();
    let witness_set = witness_set_for_testnet_message(&message, &key_refs);
    if !witness_valid_for_testnet_message(&message, &witness_set) {
        anyhow::bail!("witness set fails testnet message-hash validation");
    }
    let tx = PublicTransaction::new(message, witness_set);
    let hash = wallet
        .sequencer_client
        .send_transaction(NSSATransaction::Public(tx))
        .await
        .context("sendTransaction")?;
    Ok(format!("{hash}"))
}
