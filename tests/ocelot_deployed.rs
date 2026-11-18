//! November 18 deployed ELF, original fresh accounts and explicit historical replays.
#![cfg(feature = "reference-simulation")]
#[allow(dead_code)]
mod support;
use std::path::Path;
use titan_integration_template::{
    ocelot::{self, OcelotVenue, USDC, codec::u64_at, simulation::USER},
    trading_venue::{FromAccount, QuoteRequest, SwapType, TradingVenue},
};
const MARKETS: [&str; 2] = [
    "AuvLduWd2g2kmD4kLTGH4c3jf7XqfA9qBUDb4Lmps7kM",
    "CNEcZ2Cf6bqQTb8r9BSyZQb8o47Qa6Fd72Z1dj5BDGqp",
];
const FRESH: &str = "tests/fixtures/sol-normal-470782801.bin.gz";
async fn load(cache: &support::Snapshot, market: usize) -> OcelotVenue {
    let key = MARKETS[market].parse().unwrap();
    let mut v = OcelotVenue::from_account(&key, cache.0[&key].as_ref().unwrap()).unwrap();
    v.update_state(cache).await.unwrap();
    v.load_reference_oracle(cache).await.unwrap();
    v
}
fn request(v: &OcelotVenue, mint: bool, amount: u64) -> QuoteRequest {
    let lt = v.state().unwrap().market.mint;
    QuoteRequest {
        input_mint: if mint { USDC } else { lt },
        output_mint: if mint { lt } else { USDC },
        amount,
        swap_type: SwapType::ExactIn,
    }
}
#[tokio::test]
async fn actual_deployed_program_executes_full_sampled_ranges() {
    use sha2::{Digest, Sha256};
    let fresh = support::Snapshot::read(Path::new(FRESH));
    let p = fresh.0[&ocelot::PROGRAM_ID].as_ref().unwrap();
    let key = solana_pubkey::Pubkey::new_from_array(p.data[4..36].try_into().unwrap());
    let data = &fresh.0[&key].as_ref().unwrap().data;
    assert_eq!(u64_at(data, 4).unwrap(), 470782007);
    assert_eq!(
        format!("{:x}", Sha256::digest(&data[45..])),
        "bae5099f673264794a1e8cbea540b75e9f229aa51b7d7cb4dfcf432685e59ca5"
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(&data[45..45 + 1593280])),
        "3bb3a7bb74aee8fb84e831f124ad798df97d39305f9ca60106886ad3bee1a9e6"
    );
    assert!(data[45 + 1593280..].iter().all(|&b| b == 0));
    for market in MARKETS {
        let key = market.parse().unwrap();
        assert_eq!(
            ocelot::codec::u16_at(&fresh.0[&key].as_ref().unwrap().data, 397).unwrap(),
            0
        );
    }
    let mut executed = 0;
    for (file, market) in [
        ("sol3l-465928618", 0),
        ("sol2s-465928707", 1),
        ("sol3l-467430913", 0),
        ("sol2s-467431351", 1),
        ("sol-normal-470602378", 0),
        ("sol-normal-470602378", 1),
        ("sol-locked-468615258", 0),
        ("sol-locked-468615258", 1),
        ("sol-crossed-468615690", 0),
        ("sol-crossed-468615690", 1),
        ("sol-normal-470782801", 0),
        ("sol-normal-470782801", 1),
    ] {
        let path = Path::new("tests/fixtures").join(format!("{file}.bin.gz"));
        // Fresh replay uses the original deployed account bytes, including padding.
        let cache = if file == "sol-normal-470782801" {
            support::Snapshot::read(&path)
        } else {
            support::Snapshot::deployed(&path)
        };
        let v = load(&cache, market).await;
        for mint in [true, false] {
            let (lo, hi) = v.bounds(u8::from(!mint), u8::from(mint)).unwrap();
            println!("DEPLOYED {file}/{market} mint={mint} bounds={lo}..{hi}");
            for k in 0..=60 {
                let amount = lo + (hi - lo) * k / 60;
                let r = request(&v, mint, amount);
                let q = v.quote(r.clone()).unwrap();
                let ix = v.build_with_minimum(&r, USER, q.expected_output).unwrap();
                let (e, post) = v
                    .execution_oracle()
                    .unwrap()
                    .execute_with_accounts(ix, r.input_mint, r.output_mint, amount)
                    .unwrap_or_else(|e| panic!("{file}/{market}/{mint}/{amount}: {e}"));
                assert!(e.output >= q.expected_output);
                assert_eq!(e.output, v.reference_quote(&r).unwrap().output);
                let s = v.state().unwrap();
                let after = ocelot::codec::position(&post[&s.config.trader], &s.config).unwrap();
                let order = ocelot::quote::order(s, amount, mint).unwrap();
                assert_eq!((after.base - s.position.base).unsigned_abs(), order.lots);
                if !mint {
                    assert!(
                        (s.position.base.unsigned_abs() - after.base.unsigned_abs()) as u128
                            * s.market.shares as u128
                            >= s.position.base.unsigned_abs() as u128 * amount as u128
                    );
                }
                executed += 1;
            }
        }
    }
    assert_eq!(executed, 1464);
    println!(
        "DEPLOYED {executed} built instructions: zero reverts/overquotes; proportional closes match"
    );
}

#[tokio::test]
async fn configured_mint_ceiling_is_respected_without_coupling_redemptions() {
    let mut executed = 0;
    for market in 0..2 {
        for ceiling in [0u16, 1, 25, 300, 1000, 1001] {
            let mut cache = support::Snapshot::read(Path::new(FRESH));
            let key = MARKETS[market].parse().unwrap();
            cache.0.get_mut(&key).unwrap().as_mut().unwrap().data[397..399]
                .copy_from_slice(&ceiling.to_le_bytes());
            let v = load(&cache, market).await;
            for mint in [true, false] {
                if mint && ceiling > 1000 {
                    assert!(
                        v.bounds(0, 1).is_err(),
                        "invalid mint ceiling must disable minting only"
                    );
                    continue;
                }
                let s = v.state().unwrap();
                let order = ocelot::quote::order(s, 1_000_000, mint).unwrap();
                let resolved = if ceiling == 0 { 300 } else { ceiling };
                let bps = if mint {
                    resolved.saturating_sub(10).min(40)
                } else {
                    40
                } as u128;
                let expected = if order.buy {
                    s.mark.ticks as u128 * (10_000 + bps) / 10_000
                } else {
                    (s.mark.ticks as u128 * (10_000 - bps)).div_ceil(10_000)
                };
                assert_eq!(
                    order.limit_ticks as u128, expected,
                    "market={market} ceiling={ceiling} mint={mint}"
                );
                // At 1 bp no adverse tick may be quotable. All other cases must execute.
                let bounds = v.bounds(u8::from(!mint), u8::from(mint));
                if mint && ceiling == 1 && bounds.is_err() {
                    continue;
                }
                let (lo, hi) = bounds.unwrap();
                let amount = 10_000_000.clamp(lo, hi);
                let r = request(&v, mint, amount);
                let q = v.quote(r.clone()).unwrap();
                let ix = v.build_with_minimum(&r, USER, q.expected_output).unwrap();
                let e = v
                    .execution_oracle()
                    .unwrap()
                    .execute(ix, r.input_mint, r.output_mint, amount)
                    .unwrap();
                assert!(e.output >= q.expected_output);
                executed += 1;
            }
        }
    }
    assert!(executed >= 20);
    println!("DEPLOYED governance cases: {executed} executed, invalid ceilings disable mints only");
}

#[tokio::test]
async fn keeper_price_policy_does_not_change_user_quotes_or_instructions() {
    for market in 0..2 {
        let original = support::Snapshot::read(Path::new(FRESH));
        let baseline = load(&original, market).await;
        for keeper_bps in [1u16, 25, 50, 200] {
            let mut cache = support::Snapshot::read(Path::new(FRESH));
            let key = MARKETS[market].parse().unwrap();
            let finance = ocelot::instruction::pda(b"finance_v1", key);
            cache.0.get_mut(&finance).unwrap().as_mut().unwrap().data[59..61]
                .copy_from_slice(&keeper_bps.to_le_bytes());
            let v = load(&cache, market).await;
            for mint in [true, false] {
                let (lo, hi) = baseline.bounds(u8::from(!mint), u8::from(mint)).unwrap();
                assert_eq!(v.bounds(u8::from(!mint), u8::from(mint)).unwrap(), (lo, hi));
                let r = request(&v, mint, 10_000_000.clamp(lo, hi));
                let q = v.quote(r.clone()).unwrap();
                assert_eq!(
                    q.expected_output,
                    baseline.quote(r.clone()).unwrap().expected_output
                );
                let ix = v.build_with_minimum(&r, USER, q.expected_output).unwrap();
                assert_eq!(
                    ix,
                    baseline
                        .build_with_minimum(&r, USER, q.expected_output)
                        .unwrap()
                );
                v.execution_oracle()
                    .unwrap()
                    .execute(ix, r.input_mint, r.output_mint, r.amount)
                    .unwrap();
            }
        }
        // User flows still require a valid FinanceState, even though the keeper
        // price band is no longer the user IOC bound.
        for invalid in [0u16, 1000] {
            let mut cache = support::Snapshot::read(Path::new(FRESH));
            let key = MARKETS[market].parse().unwrap();
            let finance = ocelot::instruction::pda(b"finance_v1", key);
            cache.0.get_mut(&finance).unwrap().as_mut().unwrap().data[59..61]
                .copy_from_slice(&invalid.to_le_bytes());
            let mut v = OcelotVenue::from_account(&key, cache.0[&key].as_ref().unwrap()).unwrap();
            assert!(v.update_state(&cache).await.is_err());
            assert!(!v.initialized());
        }
    }
}

#[tokio::test]
#[ignore = "requires the built template router SBF"]
async fn deployed_program_executes_template_router_legs() {
    let elf = std::fs::read("program-template/target/deploy/titan_v3_venue_template.so").unwrap();
    for market in 0..2 {
        let cache = support::Snapshot::read(Path::new(FRESH));
        let v = load(&cache, market).await;
        for mint in [true, false] {
            let (lo, hi) = v.bounds(u8::from(!mint), u8::from(mint)).unwrap();
            for amount in [lo, lo + (hi - lo) / 2, hi] {
                let r = request(&v, mint, amount);
                let q = v.quote(r.clone()).unwrap();
                let (bank, ix) = support::routed(&v, &r, &elf, Some(12_345));
                let e = bank
                    .execute(ix, r.input_mint, r.output_mint, amount)
                    .unwrap();
                assert!(e.output >= q.expected_output);
                assert_eq!(e.output, v.reference_quote(&r).unwrap().output);
            }
        }
    }
}
