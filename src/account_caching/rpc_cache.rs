//! Fresh RPC batches for tooling, with explicit immutable snapshots for replay.
//! Titan's production AccountsCache remains supplied by the integration host.
use async_trait::async_trait;
use solana_account::Account;
use solana_account_decoder_client_types::UiAccountEncoding;
use solana_client::rpc_config::RpcAccountInfoConfig;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};
use tokio::sync::Mutex;

use crate::account_caching::{AccountCacheError, AccountsCache};
fn invalid(message: &str) -> AccountCacheError {
    AccountCacheError::FailedToFetchAccount(std::io::Error::other(message).into())
}

/// One bank response. Known absent accounts return None; undiscovered keys error
/// rather than silently fetching newer data or pretending an account is absent.
#[derive(Debug)]
pub struct RpcSnapshot {
    slot: u64,
    accounts: HashMap<Pubkey, Option<Account>>,
}
impl RpcSnapshot {
    pub fn slot(&self) -> u64 {
        self.slot
    }
}
#[async_trait]
impl AccountsCache for RpcSnapshot {
    async fn get_account(&self, key: &Pubkey) -> Result<Option<Account>, AccountCacheError> {
        self.accounts
            .get(key)
            .cloned()
            .ok_or_else(|| invalid("account outside pinned RPC snapshot"))
    }
    async fn get_accounts(
        &self,
        keys: &[Pubkey],
    ) -> Result<Vec<Option<Account>>, AccountCacheError> {
        keys.iter()
            .map(|key| {
                self.accounts
                    .get(key)
                    .cloned()
                    .ok_or_else(|| invalid("account outside pinned RPC snapshot"))
            })
            .collect()
    }
}

/// Fresh batches atomically replace the latest cached generation. Reads never
/// assemble a map from several generations. Retain an RpcSnapshot when several
/// calls (quote + simulation, including clock) must use identical data.
pub struct RpcClientCache {
    rpc_client: RpcClient,
    latest: RwLock<Option<Arc<RpcSnapshot>>>,
    // Serialize RPC fetch/publication, including single-account misses. The
    // high-water mark survives reset_cache; out-of-order nodes cannot rewind it.
    fetch: Mutex<u64>,
}
impl RpcClientCache {
    pub fn new(rpc_client: RpcClient) -> Self {
        Self {
            rpc_client,
            latest: RwLock::new(None),
            fetch: Mutex::new(0),
        }
    }
    pub fn reset_cache(&mut self) {
        *self.latest.get_mut().expect("RPC snapshot lock poisoned") = None;
    }
    /// Diagnostic read of the current batch only. Not-present and known-absent
    /// both map to None here. Use snapshot() for quote/simulation replay.
    pub fn get_multiple(&self, keys: &[Pubkey]) -> Vec<Option<Account>> {
        let latest = self.latest.read().expect("RPC snapshot lock poisoned");
        keys.iter()
            .map(|key| {
                latest
                    .as_ref()
                    .and_then(|s| s.accounts.get(key))
                    .cloned()
                    .flatten()
            })
            .collect()
    }
    /// Capture every requested key in one response, without reusing cache hits.
    pub async fn snapshot(&self, keys: &[Pubkey]) -> Result<Arc<RpcSnapshot>, AccountCacheError> {
        let mut floor = self.fetch.lock().await;
        self.fetch_snapshot(keys, &mut floor).await
    }
    async fn fetch_snapshot(
        &self,
        keys: &[Pubkey],
        floor: &mut u64,
    ) -> Result<Arc<RpcSnapshot>, AccountCacheError> {
        if keys.is_empty() || keys.len() > 100 {
            return Err(invalid("RPC snapshot requires 1..=100 accounts"));
        }
        let response = self
            .rpc_client
            .get_multiple_accounts_with_config(
                keys,
                RpcAccountInfoConfig {
                    encoding: Some(UiAccountEncoding::Base64Zstd),
                    commitment: Some(self.rpc_client.commitment()),
                    min_context_slot: Some(*floor),
                    ..Default::default()
                },
            )
            .await
            .map_err(AccountCacheError::FailedToFetchAccount)?;
        if response.value.len() != keys.len() {
            return Err(invalid("RPC returned wrong account count"));
        }
        if response.context.slot < *floor {
            return Err(invalid("RPC snapshot slot moved backwards"));
        }
        let snapshot = Arc::new(RpcSnapshot {
            slot: response.context.slot,
            accounts: keys.iter().copied().zip(response.value).collect(),
        });
        *self
            .latest
            .write()
            .map_err(|_| AccountCacheError::FailedToAcquireWriteLock)? = Some(snapshot.clone());
        *floor = snapshot.slot;
        Ok(snapshot)
    }
}
#[async_trait]
impl AccountsCache for RpcClientCache {
    async fn get_account(&self, key: &Pubkey) -> Result<Option<Account>, AccountCacheError> {
        let mut floor = self.fetch.lock().await;
        if let Some(value) = self
            .latest
            .read()
            .map_err(|_| AccountCacheError::FailedToAcquireReadLock)?
            .as_ref()
            .and_then(|s| s.accounts.get(key))
            .cloned()
        {
            return Ok(value);
        }
        self.fetch_snapshot(&[*key], &mut floor)
            .await?
            .get_account(key)
            .await
    }
    async fn get_accounts(
        &self,
        keys: &[Pubkey],
    ) -> Result<Vec<Option<Account>>, AccountCacheError> {
        if keys.is_empty() {
            return Ok(Vec::new());
        }
        self.snapshot(keys).await?.get_accounts(keys).await
    }
}
