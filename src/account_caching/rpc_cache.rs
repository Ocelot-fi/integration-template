//! RPC-backed account access for local integration tests and tooling.
//!
//! Single-account reads may reuse the cache. Each `get_accounts` call instead
//! requests the ENTIRE batch from one RPC response, including previously cached
//! keys and the clock. This prevents discovery reads from poisoning a later
//! quote snapshot. Titan's production cache is supplied by the integration host.

use async_trait::async_trait;
use dashmap::DashMap;
use solana_account::Account;
use solana_pubkey::Pubkey;
use solana_rpc_client::nonblocking::rpc_client::RpcClient;

use crate::account_caching::{AccountCacheError, AccountsCache};

/// Internal alias for the in-memory account cache.
/// Stores `Some(Account)` for found accounts and `None` for known-missing accounts.
///
/// Using `Option<Account>` avoids retrying missing accounts on every request.
type AccountCache = DashMap<Pubkey, Option<Account>>;

/// A caching layer around a Solana RPC client.
///
/// The cache performs the following optimizations:
///
/// - **Single-account fetch**: Cache hits avoid RPC calls entirely.
/// - **Multi-account fetch**: Refreshes all requested keys in one RPC call.
///   Cache hits never replace values from that response.
/// - **Caching negative lookups**: Accounts that consistently return `None` are also stored.
/// - **Thread-safe reads/writes** using `DashMap`.
pub struct RpcClientCache {
    rpc_client: RpcClient,
    cache: AccountCache,
}

impl RpcClientCache {
    /// Construct a new RPC cache from an existing `RpcClient`.
    pub fn new(rpc_client: RpcClient) -> Self {
        let cache = AccountCache::default();
        Self { rpc_client, cache }
    }

    /// Clear all cached entries.
    ///
    /// Useful when a system update or transaction batch invalidates local state.
    pub fn reset_cache(&mut self) {
        self.cache.clear();
    }

    /// Retrieve multiple accounts from the cache without making RPC requests.
    ///
    /// For each pubkey:
    /// - If present in the cache → returned immediately.
    /// - If absent → `None` is returned.
    ///
    /// This does **not** fetch from RPC; it only reads cached values. It is not
    /// a coherent snapshot API; quote preparation must use `get_accounts`.
    pub fn get_multiple(&self, pubkeys: &[Pubkey]) -> Vec<Option<Account>> {
        let mut result = Vec::with_capacity(pubkeys.len());
        pubkeys.iter().for_each(|key| {
            if let Some(value) = self.cache.get(key) {
                result.push(value.clone());
            } else {
                result.push(None);
            }
        });

        result
    }
}

#[async_trait]
impl AccountsCache for RpcClientCache {
    /// Get a single account by pubkey.
    ///
    /// - Cache hit → returned immediately.
    /// - Cache miss → RPC call made, then result cached.
    ///
    /// Errors are converted into `AccountCacheError`.
    async fn get_account(&self, pubkey: &Pubkey) -> Result<Option<Account>, AccountCacheError> {
        if let Some(account) = self.cache.get(pubkey) {
            return Ok(account.to_owned());
        }

        let response: Account = self
            .rpc_client
            .get_account(pubkey)
            .await
            .map_err(AccountCacheError::FailedToFetchAccount)?;

        // Cache positive lookup
        self.cache.insert(*pubkey, Some(response.clone()));

        Ok(Some(response))
    }

    /// Refresh one complete snapshot, rather than combining cache generations.
    /// Requests over the RPC's 100-account limit fail; splitting into multiple
    /// requests would silently lose the same-snapshot guarantee.
    async fn get_accounts(
        &self,
        pubkeys: &[Pubkey],
    ) -> Result<Vec<Option<Account>>, AccountCacheError> {
        if pubkeys.is_empty() {
            return Ok(Vec::new());
        }
        if pubkeys.len() > 100 {
            return Err(AccountCacheError::FailedToFetchAccount(
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "coherent RPC batch exceeds 100 accounts",
                )
                .into(),
            ));
        }
        let response = self
            .rpc_client
            .get_multiple_accounts(pubkeys)
            .await
            .map_err(AccountCacheError::FailedToFetchAccount)?;
        if response.len() != pubkeys.len() {
            return Err(AccountCacheError::FailedToFetchAccount(
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "RPC returned wrong account count",
                )
                .into(),
            ));
        }
        for (key, account) in pubkeys.iter().zip(&response) {
            self.cache.insert(*key, account.clone());
        }
        // Return this response directly, not a second read of the mutable cache.
        Ok(response)
    }
}
