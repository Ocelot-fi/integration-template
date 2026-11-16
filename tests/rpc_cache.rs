//! Exercise the actual RPC-backed cache with a deterministic in-process sender.
//! No network: the response generation stands in for one bank snapshot.
use async_trait::async_trait;
use serde_json::{Value, json};
use solana_client::{client_error, rpc_request::RpcRequest};
use solana_pubkey::Pubkey;
use solana_rpc_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_sender::{RpcSender, RpcTransportStats},
};
use std::sync::{Arc, Mutex};
use titan_integration_template::account_caching::{AccountsCache, rpc_cache::RpcClientCache};

#[derive(Default)]
struct Responses {
    generation: u64,
    absent: Option<Pubkey>,
    fail: bool,
    short: bool,
    batches: Vec<Vec<Pubkey>>,
    floors: Vec<u64>,
    in_flight: bool,
}
struct Sender(Arc<Mutex<Responses>>);
#[async_trait]
impl RpcSender for Sender {
    async fn send(&self, request: RpcRequest, params: Value) -> client_error::Result<Value> {
        assert_eq!(params[1]["encoding"], "base64+zstd");
        {
            let mut state = self.0.lock().unwrap();
            assert!(!state.in_flight, "RPC fetches must be serialized");
            state.in_flight = true;
        }
        tokio::task::yield_now().await;
        let mut state = self.0.lock().unwrap();
        state.in_flight = false;
        if state.fail {
            return Err(std::io::Error::other("controlled transport failure").into());
        }
        state
            .floors
            .push(params[1]["minContextSlot"].as_u64().unwrap());
        let value = |key: Pubkey| {
            if state.absent == Some(key) {
                Value::Null
            } else {
                json!({"lamports":state.generation, "owner":Pubkey::default().to_string(),
                    "data":["", "base64"], "executable":false, "rentEpoch":0})
            }
        };
        let values = match request {
            RpcRequest::GetAccountInfo => value(params[0].as_str().unwrap().parse().unwrap()),
            RpcRequest::GetMultipleAccounts => {
                let keys: Vec<Pubkey> = params[0]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| v.as_str().unwrap().parse().unwrap())
                    .collect();
                let mut values: Vec<_> = keys.iter().map(|k| value(*k)).collect();
                if state.short {
                    values.pop();
                }
                state.batches.push(keys);
                json!(values)
            }
            _ => panic!("unexpected RPC request: {request:?}"),
        };
        Ok(json!({"context":{"slot":state.generation}, "value":values}))
    }
    fn get_transport_stats(&self) -> RpcTransportStats {
        RpcTransportStats::default()
    }
    fn url(&self) -> String {
        "http://offline.invalid".into()
    }
}
fn cache() -> (RpcClientCache, Arc<Mutex<Responses>>) {
    let state = Arc::new(Mutex::new(Responses {
        generation: 100,
        ..Responses::default()
    }));
    let rpc = RpcClient::new_sender(Sender(state.clone()), Default::default());
    (RpcClientCache::new(rpc), state)
}

#[tokio::test]
async fn final_batch_replaces_discovery_hits_and_preserves_order_and_absence() {
    let (cache, state) = cache();
    let a = Pubkey::new_unique();
    let b = Pubkey::new_unique();
    assert_eq!(cache.get_account(&a).await.unwrap().unwrap().lamports, 100);
    state.lock().unwrap().generation = 101;
    // Cached index + uncached buffer/clock must all come from the new response.
    let keys = [b, a, b];
    let batch = cache.get_accounts(&keys).await.unwrap();
    assert!(batch.iter().all(|a| a.as_ref().unwrap().lamports == 101));
    assert_eq!(state.lock().unwrap().batches, vec![vec![a], keys.to_vec()]);
    {
        let mut s = state.lock().unwrap();
        s.generation = 102;
        s.absent = Some(a);
    }
    let batch = cache.get_accounts(&[a, b]).await.unwrap();
    assert!(
        batch[0].is_none(),
        "a deleted account cannot retain its cached value"
    );
    assert_eq!(batch[1].as_ref().unwrap().lamports, 102);
    {
        let mut s = state.lock().unwrap();
        s.generation = 103;
        s.absent = None;
    }
    assert_eq!(
        cache.get_accounts(&[a]).await.unwrap()[0]
            .as_ref()
            .unwrap()
            .lamports,
        103
    );
}

#[tokio::test]
async fn failed_or_invalid_batch_never_returns_old_cached_accounts() {
    let (cache, state) = cache();
    let key = Pubkey::new_unique();
    cache.get_account(&key).await.unwrap();
    state.lock().unwrap().fail = true;
    assert!(cache.get_accounts(&[key]).await.is_err());
    {
        let mut s = state.lock().unwrap();
        s.fail = false;
        s.short = true;
    }
    assert!(cache.get_accounts(&[key]).await.is_err());
    state.lock().unwrap().short = false;
    let before = state.lock().unwrap().batches.len();
    assert!(cache.get_accounts(&[key; 101]).await.is_err());
    assert!(cache.get_accounts(&[]).await.unwrap().is_empty());
    assert_eq!(state.lock().unwrap().batches.len(), before);
}

#[tokio::test]
async fn pinned_snapshot_survives_refresh_and_slot_regression_is_rejected() {
    let (mut cache, state) = cache();
    let a = Pubkey::new_unique();
    let b = Pubkey::new_unique();
    state.lock().unwrap().absent = Some(b);
    let pinned = cache.snapshot(&[a, b]).await.unwrap();
    state.lock().unwrap().generation = 101;
    cache.get_accounts(&[a]).await.unwrap();
    assert_eq!(pinned.slot(), 100);
    assert_eq!(pinned.get_account(&a).await.unwrap().unwrap().lamports, 100);
    assert!(pinned.get_account(&b).await.unwrap().is_none());
    assert!(pinned.get_account(&Pubkey::new_unique()).await.is_err());
    cache.reset_cache();
    state.lock().unwrap().generation = 99;
    assert!(cache.get_accounts(&[a]).await.is_err());
    assert_eq!(state.lock().unwrap().floors, vec![0, 100, 101]);
    assert!(cache.get_multiple(&[a])[0].is_none());
    state.lock().unwrap().generation = 102;
    cache.get_account(&a).await.unwrap();
    assert_eq!(state.lock().unwrap().floors.last(), Some(&101));
}

#[tokio::test]
async fn concurrent_batches_publish_only_whole_generations() {
    let (cache, _) = cache();
    let a = Pubkey::new_unique();
    let b = Pubkey::new_unique();
    let a_keys = [a];
    let b_keys = [b];
    let (one, two) = tokio::join!(cache.snapshot(&a_keys), cache.snapshot(&b_keys));
    assert!(one.unwrap().get_account(&a).await.unwrap().is_some());
    assert!(two.unwrap().get_account(&b).await.unwrap().is_some());
    let current = cache.get_multiple(&[a, b]);
    assert_eq!(current.iter().filter(|a| a.is_some()).count(), 1);
}
