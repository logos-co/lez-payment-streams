use std::path::Path;

use anyhow::{Context as _, Result};
use common::transaction::NSSATransaction;
use nssa::{
    program_deployment_transaction::{Message as DeployMessage, ProgramDeploymentTransaction},
    public_transaction::{Message, WitnessSet},
    Account, AccountId, PrivateKey, ProgramId, PublicTransaction,
};
use reqwest::Client;
use serde::Deserialize;
use serde_json::{json, Value};
use url::Url;

#[derive(Clone)]
pub struct LegacySequencerClient {
    client: Client,
    url: Url,
}

impl LegacySequencerClient {
    pub fn from_wallet_config(wallet_config: &Path) -> Result<Self> {
        let cfg: Value =
            serde_json::from_str(&std::fs::read_to_string(wallet_config).context("read wallet config")?)?;
        let addr = cfg
            .get("sequencer_addr")
            .and_then(|v| v.as_str())
            .context("wallet_config missing sequencer_addr")?;
        let url = Url::parse(addr).context("parse sequencer_addr")?;
        Ok(Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .context("build HTTP client")?,
            url,
        })
    }

    async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let request = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1,
        });
        let response = self
            .client
            .post(self.url.clone())
            .json(&request)
            .send()
            .await
            .context("sequencer HTTP POST")?
            .json::<Value>()
            .await
            .context("sequencer JSON response")?;

        if let Some(result) = response.get("result") {
            return Ok(result.clone());
        }

        if let Some(err) = response.get("error") {
            anyhow::bail!("sequencer RPC {method} error: {err}");
        }

        anyhow::bail!("sequencer RPC {method} missing result: {response}");
    }

    pub async fn last_block(&self) -> Result<u64> {
        let result = self.call("get_last_block", json!({})).await?;
        result
            .get("last_block")
            .and_then(|v| v.as_u64())
            .context("get_last_block result.last_block")
    }

    pub async fn get_accounts_nonces(&self, account_ids: Vec<AccountId>) -> Result<Vec<u128>> {
        let req = serde_json::to_value(json!({ "account_ids": account_ids }))?;
        let result = self.call("get_accounts_nonces", req).await?;
        let resp: NoncesResponse = serde_json::from_value(result).context("decode nonces")?;
        Ok(resp.nonces)
    }

    pub async fn get_account(&self, account_id: AccountId) -> Result<Account> {
        let req = serde_json::to_value(json!({ "account_id": account_id }))?;
        let result = self.call("get_account", req).await?;
        let resp: AccountResponse = serde_json::from_value(result).context("decode account")?;
        Ok(resp.account)
    }

    pub async fn send_nssa_tx(&self, tx: NSSATransaction) -> Result<String> {
        let bytes = borsh::to_vec(&tx).context("borsh NSSATransaction")?;
        use base64::{engine::general_purpose, Engine as _};
        let b64 = general_purpose::STANDARD.encode(bytes);
        let result = self
            .call("send_tx", json!({ "transaction": b64 }))
            .await?;
        let resp: SendTxResponse = serde_json::from_value(result).context("decode send_tx")?;
        Ok(hex::encode(resp.tx_hash))
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

pub async fn submit_public_with_wallet(
    legacy: &LegacySequencerClient,
    wallet: &wallet::WalletCore,
    payload: &super::submit::SubmitPayload,
    program_id: ProgramId,
    account_ids: Vec<AccountId>,
    instruction_words: Vec<u32>,
) -> Result<String> {
    let nonces: Vec<nssa_core::account::Nonce> = legacy
        .get_accounts_nonces(account_ids.clone())
        .await?
        .into_iter()
        .map(Into::into)
        .collect();

    let mut private_keys = Vec::new();
    for (account_id, needs_sign) in account_ids.iter().zip(payload.signing_requirements.iter()) {
        if *needs_sign {
            let key = wallet
                .storage()
                .user_data
                .get_pub_account_signing_key(*account_id)
                .ok_or_else(|| anyhow::anyhow!("signing key not found for {account_id:?}"))?;
            private_keys.push(key);
        }
    }

    let message = Message::new_preserialized(program_id, account_ids, nonces, instruction_words);
    let key_refs: Vec<&PrivateKey> = private_keys.iter().copied().collect();
    let witness_set = WitnessSet::for_message(&message, &key_refs);
    let tx = PublicTransaction::new(message, witness_set);
    legacy.send_public_tx(tx).await
}

#[derive(Debug, Deserialize)]
struct NoncesResponse {
    nonces: Vec<u128>,
}

#[derive(Debug, Deserialize)]
struct AccountResponse {
    account: Account,
}

#[derive(Debug, Deserialize)]
struct SendTxResponse {
    tx_hash: [u8; 32],
}
