//! Original ANT state replay; Phoenix simulation owns oracle/action eligibility.
#![cfg(feature = "reference-simulation")]
#[allow(dead_code)]
mod support;
use std::path::Path;
use titan_integration_template::{
    ocelot::{
        self, OcelotVenue, USDC,
        codec::{key, u16_at},
        simulation::USER,
    },
    trading_venue::{FromAccount, QuoteRequest, SwapType, TradingVenue},
};

const CAPTURE: &str = "tests/fixtures/ant-normal-471413600.bin.gz";
const MARKETS: [&str; 2] = [
    "436HSbzkKcv5h6k4R5yDXP6vktvwYKHt7yy3gJPU1rtj",
    "ELVLD8SqR3e4V41f5q38bKqMj8tPZyiSz1AFLNgCkysj",
];
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
async fn ant_original_capture_executes_both_directions() {
    use sha2::{Digest, Sha256};
    let cache = support::Snapshot::read(Path::new(CAPTURE));
    assert_eq!(cache.0.len(), 48);
    for (program, hash) in [
        (
            ocelot::PROGRAM_ID,
            "bae5099f673264794a1e8cbea540b75e9f229aa51b7d7cb4dfcf432685e59ca5",
        ),
        (
            ocelot::PHOENIX,
            "302004ae6d89b08f905ecda265d3fca77ed43ab30475396c150b3b70b91162f4",
        ),
    ] {
        let k = key(&cache.0[&program].as_ref().unwrap().data, 4).unwrap();
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(&cache.0[&k].as_ref().unwrap().data[45..])
            ),
            hash
        );
    }
    let mut executed = 0;
    for m in 0..2 {
        let v = load(&cache, m).await;
        assert_eq!(v.state().unwrap().config.asset, 110);
        assert_eq!(v.state().unwrap().clock.slot, 471413600);
        for mint in [true, false] {
            let (lo, hi) = v.bounds(u8::from(!mint), u8::from(mint)).unwrap();
            println!("ANT market={m} mint={mint} bounds={lo}..{hi}");
            let mut amounts: Vec<_> = (0..=60).map(|k| lo + (hi - lo) * k / 60).collect();
            if mint {
                amounts.extend([
                    1_000_000,
                    10_000_000,
                    100_000_000,
                    500_000_000,
                    1_000_000_000,
                    5_000_000_000,
                ]);
            } else {
                amounts.push(v.state().unwrap().actual_supply);
            }
            for amount in amounts {
                let r = request(&v, mint, amount);
                let q = v.quote(r.clone()).unwrap();
                let ix = v.build_with_minimum(&r, USER, q.expected_output).unwrap();
                let e = v
                    .execution_oracle()
                    .unwrap()
                    .execute(ix, r.input_mint, r.output_mint, amount)
                    .unwrap_or_else(|e| panic!("market={m} mint={mint} amount={amount}: {e}"));
                assert!(e.output >= q.expected_output);
                assert_eq!(e.input_remaining, 0);
                executed += 1;
            }
        }
    }
    assert_eq!(executed, 258);
    println!("ANT original capture: {executed} executed, zero overquotes or failures");
}

#[tokio::test]
async fn ant_large_mint_then_redeem_uses_actual_post_trade_state() {
    for m in 0..2 {
        let original = support::Snapshot::read(Path::new(CAPTURE));
        let v = load(&original, m).await;
        let r = request(&v, true, 1_000_000_000);
        let q = v.quote(r.clone()).unwrap();
        let ix = v.build_with_minimum(&r, USER, q.expected_output).unwrap();
        let (minted, post) = v
            .execution_oracle()
            .unwrap()
            .execute_with_accounts(ix, r.input_mint, r.output_mint, r.amount)
            .unwrap();
        let mut cache = support::Snapshot(original.0.clone());
        for (k, a) in post {
            cache.0.insert(k, Some(a));
        }
        let v = load(&cache, m).await;
        for amount in [minted.output / 4, minted.output / 2, minted.output] {
            let r = request(&v, false, amount);
            let q = v.quote(r.clone()).unwrap();
            let ix = v.build_with_minimum(&r, USER, q.expected_output).unwrap();
            let e = v
                .execution_oracle()
                .unwrap()
                .execute(ix, r.input_mint, r.output_mint, amount)
                .unwrap();
            assert!(e.output >= q.expected_output);
        }
    }
}

#[tokio::test]
async fn ant_session_flags_and_oracle_policy_do_not_gate_quotes() {
    let original = support::Snapshot::read(Path::new(CAPTURE));
    for m in 0..2 {
        let baseline = load(&original, m).await;
        let d = &original.0[&ocelot::PERP_MAP].as_ref().unwrap().data;
        let count = u32::from_le_bytes(d[32..36].try_into().unwrap()) as usize;
        let o = (0..count)
            .map(|i| 64 + i * 1584)
            .find(|&o| d[o + 1450] == 0 && u16_at(d, o + 944).unwrap() == 110)
            .unwrap();
        assert_eq!(d[o + 1456], 1);
        assert_eq!(&d[o + 811..o + 813], &[10, 10]);
        for flags in [0u8, 1, 3, 5, 7] {
            let mut cache = support::Snapshot(original.0.clone());
            cache
                .0
                .get_mut(&ocelot::PERP_MAP)
                .unwrap()
                .as_mut()
                .unwrap()
                .data[o + 1456] = flags;
            let v = load(&cache, m).await;
            for mint in [true, false] {
                let r = request(&v, mint, 1_000_000);
                let q = v.quote(r.clone()).unwrap();
                assert_eq!(
                    q.expected_output,
                    baseline.quote(r.clone()).unwrap().expected_output
                );
                // These are controlled flag changes, not real session captures.
                let e = v.reference_quote(&r);
                println!(
                    "ANT flag control market={m} flags={flags} mint={mint} quote={} execution={e:?}",
                    q.expected_output
                );
                assert!(e.unwrap().output >= q.expected_output);
            }
        }
        // A stale valuation is distinct from action eligibility and must still
        // invalidate the cached state instead of pricing from a missing mark.
        let mut cache = support::Snapshot(original.0.clone());
        cache
            .0
            .get_mut(&ocelot::PERP_MAP)
            .unwrap()
            .as_mut()
            .unwrap()
            .data[o + 16..o + 24]
            .fill(0);
        let mut v = baseline;
        let error = v.update_state(&cache).await.unwrap_err().to_string();
        assert!(error.contains("stale or future Phoenix mark"), "{error}");
        assert!(!v.initialized());
    }
}

#[tokio::test]
#[ignore = "requires the built local template router SBF"]
async fn ant_executes_through_titan_template_router() {
    let elf = std::fs::read("program-template/target/deploy/titan_v3_venue_template.so").unwrap();
    let cache = support::Snapshot::read(Path::new(CAPTURE));
    for m in 0..2 {
        let v = load(&cache, m).await;
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
            }
        }
    }
}

#[tokio::test]
async fn ant_lot_edges_preserve_monotone_concave_curve_and_marginal_price() {
    let cache = support::Snapshot::read(Path::new(CAPTURE));
    let mut checked = 0;
    for m in 0..2 {
        let v = load(&cache, m).await;
        let s = v.state().unwrap();
        for mint in [true, false] {
            let (lo, hi) = v.bounds(u8::from(!mint), u8::from(mint)).unwrap();
            let lots = |a| ocelot::quote::order(s, a, mint).unwrap().lots;
            let mut amounts = vec![lo, hi];
            for target in lots(lo)..=lots(hi) {
                let (mut a, mut b) = (lo, hi);
                while a < b {
                    let mid = a + (b - a) / 2;
                    if lots(mid) >= target {
                        b = mid;
                    } else {
                        a = mid + 1;
                    }
                }
                amounts.extend([a.saturating_sub(1).max(lo), a, (a + 1).min(hi)]);
            }
            let mut seed = 7u64;
            for _ in 0..20_000 {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                amounts.push(lo + seed % (hi - lo + 1));
            }
            amounts.sort_unstable();
            amounts.dedup();
            let mut previous: Option<titan_integration_template::trading_venue::QuoteResult> = None;
            for amount in amounts {
                let q = v.quote(request(&v, mint, amount)).unwrap();
                if let Some(p) = previous {
                    assert!(q.expected_output >= p.expected_output);
                    assert!(q.price > 0.0 && q.price <= p.price * (1.0 + 1e-5));
                    let dx = (amount - p.amount) as f64;
                    let slope = (q.expected_output - p.expected_output) as f64 / dx;
                    assert!(slope <= p.price * (1.0 + 1e-5) + 2.0 / dx);
                    assert!(slope >= q.price * (1.0 - 1e-5) - 2.0 / dx);
                }
                previous = Some(q);
                checked += 1;
            }
        }
    }
    println!("ANT curve: {checked} boundary/interior quotes, no holes or shape failures");
}
