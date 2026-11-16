#![cfg(feature = "reference-simulation")]
#![allow(clippy::result_large_err)]
mod common;
#[allow(dead_code)]
mod support;
use async_trait::async_trait;
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use solana_client::{client_error, rpc_request::RpcRequest};
use solana_pubkey::Pubkey;
use solana_rpc_client::{
    nonblocking::rpc_client::RpcClient,
    rpc_sender::{RpcSender, RpcTransportStats},
};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use titan_integration_template::{
    account_caching::{AccountsCache, rpc_cache::RpcClientCache},
    ocelot::{OcelotVenue, USDC, codec::u64_at, simulation::USER},
    trading_venue::{FromAccount, QuoteRequest, SwapType, TradingVenue},
};
const POOL: &str = "AuvLduWd2g2kmD4kLTGH4c3jf7XqfA9qBUDb4Lmps7kM";
const FILE: &str = "sol3l-465928618.bin.gz";
fn req(v: &OcelotVenue, mint: bool, amount: u64) -> QuoteRequest {
    let lt = v.state().unwrap().market.mint;
    QuoteRequest {
        input_mint: if mint { USDC } else { lt },
        output_mint: if mint { lt } else { USDC },
        amount,
        swap_type: SwapType::ExactIn,
    }
}
fn at_slot(base: &support::Snapshot, slot: u64, queue: Pubkey) -> support::Snapshot {
    let mut out = support::Snapshot(base.0.clone());
    let old_slot = u64_at(
        &base.0[&solana_sdk_ids::sysvar::clock::ID]
            .as_ref()
            .unwrap()
            .data,
        0,
    )
    .unwrap();
    // Model makers refreshing unchanged prices as the bank advances, so this
    // test isolates cache coherence instead of deliberately expiring the book.
    for a in out.0.values_mut().flatten() {
        if a.data.get(..8)
            == Some(&titan_integration_template::ocelot::discriminator(
                "account:spline_collection",
            ))
        {
            let count = u32::from_le_bytes(a.data[72..76].try_into().unwrap()) as usize;
            for i in 0..count {
                let o = 112 + i * 1336 + 64;
                let updated = u64_at(&a.data, o).unwrap();
                if updated > 0 && updated <= old_slot {
                    a.data[o..o + 8].copy_from_slice(&(updated + slot - old_slot).to_le_bytes());
                }
            }
        }
    }

    out.0
        .get_mut(&solana_sdk_ids::sysvar::clock::ID)
        .unwrap()
        .as_mut()
        .unwrap()
        .data[..8]
        .copy_from_slice(&slot.to_le_bytes());
    out.0.get_mut(&queue).unwrap().as_mut().unwrap().data[48..56]
        .copy_from_slice(&slot.to_le_bytes());
    out
}
struct Sender {
    base: Arc<support::Snapshot>,
    queue: Pubkey,
    start: u64,
    calls: Arc<Mutex<Vec<Vec<Pubkey>>>>,
    override_account: Arc<Mutex<Option<(Pubkey, Value)>>>,
}
#[async_trait]
impl RpcSender for Sender {
    async fn send(&self, method: RpcRequest, params: Value) -> client_error::Result<Value> {
        assert_eq!(method, RpcRequest::GetMultipleAccounts);
        assert_eq!(params[1]["encoding"], "base64+zstd");
        let keys: Vec<Pubkey> = params[0]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().parse().unwrap())
            .collect();
        let slot = {
            let mut calls = self.calls.lock().unwrap();
            calls.push(keys.clone());
            self.start + calls.len() as u64
        };
        assert!(params[1]["minContextSlot"].as_u64().unwrap() <= slot);
        let state = at_slot(&self.base, slot, self.queue);
        let override_account = self.override_account.lock().unwrap();
        let values: Vec<Value> = keys
            .iter()
            .map(|key| {
                if let Some((target, value)) = override_account.as_ref()
                    && target == key
                {
                    return value.clone();
                }
                match state.0.get(key).and_then(Option::as_ref) {
                    None => Value::Null,
                    Some(a) => json!({
                        "lamports":a.lamports,
                        "owner":a.owner.to_string(),
                        "data":[STANDARD.encode(&a.data),"base64"],
                        "executable":a.executable,
                        "rentEpoch":a.rent_epoch
                    }),
                }
            })
            .collect();
        Ok(json!({"context":{"slot":slot},"value":values}))
    }
    fn get_transport_stats(&self) -> RpcTransportStats {
        RpcTransportStats::default()
    }
    fn url(&self) -> String {
        "http://offline.invalid".into()
    }
}
async fn fixture() -> (support::Snapshot, OcelotVenue) {
    let base = support::Snapshot::read(&Path::new("tests/fixtures").join(FILE));
    let pool = POOL.parse().unwrap();
    let mut v = OcelotVenue::from_account(&pool, base.0[&pool].as_ref().unwrap()).unwrap();
    v.update_state(&base).await.unwrap();
    (base, v)
}

// Negative control for the template's original hit-reuse policy. The underlying
// RPC source is identical; only merging old hits with new misses differs.
struct HitReusing {
    source: RpcClientCache,
    hits: tokio::sync::Mutex<std::collections::HashMap<Pubkey, Option<solana_account::Account>>>,
}
#[async_trait]
impl AccountsCache for HitReusing {
    async fn get_account(
        &self,
        key: &Pubkey,
    ) -> Result<
        Option<solana_account::Account>,
        titan_integration_template::account_caching::AccountCacheError,
    > {
        Ok(self.get_accounts(&[*key]).await?.remove(0))
    }
    async fn get_accounts(
        &self,
        keys: &[Pubkey],
    ) -> Result<
        Vec<Option<solana_account::Account>>,
        titan_integration_template::account_caching::AccountCacheError,
    > {
        let mut hits = self.hits.lock().await;
        let missing: Vec<_> = keys
            .iter()
            .filter(|k| !hits.contains_key(k))
            .copied()
            .collect();
        let values = self.source.get_accounts(&missing).await?;
        hits.extend(missing.into_iter().zip(values));
        Ok(keys.iter().map(|k| hits[k].clone()).collect())
    }
}

#[tokio::test]
async fn advancing_rpc_refresh_then_shared_simulation_pin_keeps_one_state() {
    let (base, original) = fixture().await;
    let pool = POOL.parse().unwrap();
    let queue = original.state().unwrap().exchange.withdraw_queue;
    let start = original.state().unwrap().clock.slot;
    let legacy = HitReusing {
        source: RpcClientCache::new(RpcClient::new_sender(
            Sender {
                base: Arc::new(support::Snapshot(base.0.clone())),
                queue,
                start,
                calls: Arc::new(Mutex::new(vec![])),
                override_account: Default::default(),
            },
            Default::default(),
        )),
        hits: Default::default(),
    };
    let mut rejected = OcelotVenue::from_account(&pool, base.0[&pool].as_ref().unwrap()).unwrap();
    assert!(
        rejected
            .update_state(&legacy)
            .await
            .unwrap_err()
            .to_string()
            .contains("invalid withdrawal budget")
    );
    assert!(!rejected.initialized());
    let calls = Arc::new(Mutex::new(vec![]));
    let sender = Sender {
        base: Arc::new(support::Snapshot(base.0.clone())),
        queue,
        start,
        calls: calls.clone(),
        override_account: Default::default(),
    };
    let cache = RpcClientCache::new(RpcClient::new_sender(sender, Default::default()));
    let mut v = OcelotVenue::from_account(&pool, base.0[&pool].as_ref().unwrap()).unwrap();
    v.update_state(&cache).await.unwrap();
    assert_eq!(v.state().unwrap().clock.slot, start + 3);
    assert_eq!(
        u64_at(&v.state().unwrap().accounts[&queue].data, 48).unwrap(),
        start + 3
    );
    v.update_state(&cache).await.unwrap();
    assert_eq!(v.state().unwrap().clock.slot, start + 5);
    // Actual shared-suite helper, not a parallel imitation of its pinning code.
    let (mut v, pinned) = common::pin_simulation_state(v, &cache, USER).await;
    let slot = pinned.slot();
    let requests: Vec<_> = [true, false]
        .into_iter()
        .map(|mint| req(&v, mint, 1_000_000))
        .collect();
    let outputs: Vec<_> = requests
        .iter()
        .map(|r| v.quote(r.clone()).unwrap().expected_output)
        .collect();
    let keys = v.get_required_pubkeys_for_update().unwrap();
    cache.get_accounts(&keys).await.unwrap();
    let count = calls.lock().unwrap().len();
    assert_eq!(
        u64_at(
            &pinned
                .get_account(&solana_sdk_ids::sysvar::clock::ID)
                .await
                .unwrap()
                .unwrap()
                .data,
            0
        )
        .unwrap(),
        slot
    );
    v.update_state(pinned.as_ref()).await.unwrap();
    let fixed = at_slot(&base, slot, queue);
    v.load_reference_oracle(&fixed).await.unwrap();
    for (r, output) in requests.into_iter().zip(outputs) {
        assert_eq!(v.quote(r.clone()).unwrap().expected_output, output);
        assert!(output <= v.reference_quote(&r).unwrap().output);
        let ix = v.generate_swap_instruction(r, USER).unwrap();
        pinned
            .get_accounts(&ix.accounts.iter().map(|a| a.pubkey).collect::<Vec<_>>())
            .await
            .unwrap();
    }
    assert_eq!(
        calls.lock().unwrap().len(),
        count,
        "pinned replays must not call RPC"
    );
}

#[tokio::test]
async fn unreadable_optional_throttle_declines_instead_of_becoming_unlimited() {
    let (base, mut venue) = fixture().await;
    let pool: Pubkey = POOL.parse().unwrap();
    let bucket = Pubkey::find_program_address(
        &[b"withdraw_throttle", b"market", pool.as_ref()],
        &titan_integration_template::ocelot::PROGRAM_ID,
    )
    .0;
    let override_account = Arc::new(Mutex::new(None));
    let cache = RpcClientCache::new(RpcClient::new_sender(
        Sender {
            queue: venue.state().unwrap().exchange.withdraw_queue,
            start: venue.state().unwrap().clock.slot,
            base: Arc::new(base),
            calls: Default::default(),
            override_account: override_account.clone(),
        },
        Default::default(),
    ));
    venue.update_state(&cache).await.unwrap();
    let request = req(&venue, false, 1_000_000);
    venue.quote(request.clone()).unwrap();
    *override_account.lock().unwrap() = Some((
        bucket,
        json!({
            "lamports":1, "owner":titan_integration_template::ocelot::PROGRAM_ID.to_string(),
            "data":[STANDARD.encode(b"corrupt compressed throttle"), "base64+zstd"],
            "executable":false, "rentEpoch":0
        }),
    ));
    let err = venue.update_state(&cache).await.unwrap_err();
    assert!(format!("{err:?}").contains("RPC failed to decode account"));
    assert!(!venue.initialized());
    assert!(venue.quote(request.clone()).is_err());
    assert!(venue.bounds(0, 1).is_err());
    assert!(
        venue
            .generate_swap_instruction(request.clone(), USER)
            .is_err()
    );
    // Only an actual RPC null may take the program's absent-market-bucket path.
    *override_account.lock().unwrap() = Some((bucket, Value::Null));
    venue.update_state(&cache).await.unwrap();
    assert!(!venue.state().unwrap().accounts.contains_key(&bucket));
    venue.quote(request).unwrap();
}

#[tokio::test]
async fn torn_index_buffer_declines_and_coherent_refresh_recovers() {
    let (base, original) = fixture().await;
    let s = original.state().unwrap();
    let index = s.exchange.indices[0];
    let buffer = s.exchange.buffers[0];
    let mut late = support::Snapshot(base.0.clone());
    let u32_at = |d: &[u8], o| u32::from_le_bytes(d[o..o + 4].try_into().unwrap());
    let idx = &base.0[&index].as_ref().unwrap().data;
    let mut nodes = vec![];
    let mut stack = vec![u32_at(idx, 80)];
    while let Some(n) = stack.pop() {
        if n == 0 {
            continue;
        }
        nodes.push(n);
        let o = 96 + (n as usize - 1) * 64;
        stack.extend([u32_at(idx, o), u32_at(idx, o + 4)]);
    }
    let buf = &base.0[&buffer].as_ref().unwrap().data;
    let mut entries = vec![];
    let mut stack = vec![u32_at(buf, 80)];
    while let Some(n) = stack.pop() {
        if n == 0 {
            continue;
        }
        let o = 96 + (n as usize - 1) * 88;
        entries.push(o);
        stack.extend([u32_at(buf, o), u32_at(buf, o + 4)]);
    }
    let old = u32_at(buf, entries[0] + 16);
    let new = (1..=((idx.len() - 96) / 64) as u32)
        .find(|n| !nodes.contains(n))
        .unwrap();
    // Controlled node relocation preserves the index tree and corresponding
    // position keys. It exercises our parser, not a claimed live lifecycle.
    let d = &mut late.0.get_mut(&index).unwrap().as_mut().unwrap().data;
    let a = 96 + (old as usize - 1) * 64;
    let b = 96 + (new as usize - 1) * 64;
    let record = d[a..a + 64].to_vec();
    d[b..b + 64].copy_from_slice(&record);
    for n in &nodes {
        let o = 96 + ((if *n == old { new } else { *n }) as usize - 1) * 64;
        for off in [o, o + 4, o + 8] {
            if u32_at(d, off) == old {
                d[off..off + 4].copy_from_slice(&new.to_le_bytes());
            }
        }
    }
    if u32_at(d, 80) == old {
        d[80..84].copy_from_slice(&new.to_le_bytes());
    }
    let d = &mut late.0.get_mut(&buffer).unwrap().as_mut().unwrap().data;
    for o in entries {
        if u32_at(d, o + 16) == old {
            d[o + 16..o + 20].copy_from_slice(&new.to_le_bytes());
        }
    }
    let mut torn = support::Snapshot(late.0.clone());
    torn.0.insert(index, base.0[&index].clone());
    let mut v = original.clone();
    assert!(
        v.update_state(&torn)
            .await
            .unwrap_err()
            .to_string()
            .contains("orphan active position")
    );
    assert!(!v.initialized());
    assert!(v.quote(req(&original, true, 1_000_000)).is_err());
    v.update_state(&late).await.unwrap();
    for mint in [true, false] {
        let r = req(&v, mint, 1_000_000);
        assert_eq!(
            v.quote(r.clone()).unwrap().expected_output,
            original.quote(r).unwrap().expected_output
        );
    }
}
