use std::path::Path;

use anyhow::{Context as _, Result};
use common::{transaction::LeeTransaction, HashType};
use lee::{
    program_deployment_transaction::{Message as DeployMessage, ProgramDeploymentTransaction},
    public_transaction::{Message, WitnessSet},
    Account, AccountId, PrivateKey, ProgramId, PublicTransaction,
};
use sequencer_service_rpc::{RpcClient as _, SequencerClient, SequencerClientBuilder};
use serde_json::Value;
use url::Url;
use lez_payment_streams_core::instruction_try_from_instruction_words;
use wallet::WalletCore;
use wallet::poller::TxPoller;

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

    pub async fn send_lee_tx(&self, tx: LeeTransaction) -> Result<String> {
        let hash: HashType = self
            .client
            .send_transaction(tx)
            .await
            .context("sendTransaction")?;
        Ok(format!("{hash}"))
    }

    pub async fn send_public_tx(&self, tx: PublicTransaction) -> Result<String> {
        self.send_lee_tx(LeeTransaction::Public(tx)).await
    }

    pub async fn deploy_program_bytecode(&self, bytecode: Vec<u8>) -> Result<String> {
        let message = DeployMessage::new(bytecode);
        let transaction = ProgramDeploymentTransaction::new(message);
        self.send_lee_tx(LeeTransaction::ProgramDeployment(transaction))
            .await
    }
}

pub async fn submit_public_with_wallet(
    wallet: &WalletCore,
    payload: &super::submit::SubmitPayload,
    program_id: ProgramId,
    account_ids: Vec<AccountId>,
    instruction_words: Vec<u32>,
) -> Result<String> {
    let mut signing_account_ids = Vec::new();
    let mut private_key_refs: Vec<&PrivateKey> = Vec::new();
    for (account_id, needs_sign) in account_ids.iter().zip(payload.signing_requirements.iter()) {
        if *needs_sign {
            signing_account_ids.push(*account_id);
            let key = wallet
                .get_account_public_signing_key(*account_id)
                .ok_or_else(|| anyhow::anyhow!("signing key not found for {account_id:?}"))?;
            private_key_refs.push(key);
        }
    }

    let nonces = wallet
        .get_accounts_nonces(signing_account_ids)
        .await
        .context("get_accounts_nonces")?;

    let instruction = instruction_try_from_instruction_words(&instruction_words)
        .map_err(|e| anyhow::anyhow!("decode payment-streams instruction: {e}"))?;
    let message =
        Message::try_new(program_id, account_ids, nonces, instruction).context("build message")?;
    let witness_set = WitnessSet::for_message(&message, &private_key_refs);
    if !witness_set.is_valid_for(&message) {
        anyhow::bail!("witness set fails message-hash validation");
    }
    let tx = PublicTransaction::new(message, witness_set);
    let tx_hash = wallet
        .sequencer_client
        .send_transaction(LeeTransaction::Public(tx))
        .await
        .context("sendTransaction")?;
    let poller = TxPoller::new(wallet.config(), wallet.sequencer_client.clone());
    poller
        .poll_tx(tx_hash)
        .await
        .context("confirm transaction")?;
    Ok(format!("{tx_hash}"))
}
