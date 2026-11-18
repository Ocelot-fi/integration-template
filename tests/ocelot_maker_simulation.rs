//! Native quotes use declared maker depth; Phoenix simulation owns maker solvency.
#![cfg(feature = "reference-simulation")]
#[allow(dead_code)]
mod support;
use std::path::Path;
use titan_integration_template::{
    ocelot::{
        self, OcelotVenue, USDC,
        codec::{key, u16_at, u64_at},
        simulation::USER,
    },
    trading_venue::{FromAccount, QuoteRequest, SwapType, TradingVenue},
};

const CAPTURE: &str = "tests/fixtures/sol-normal-471112973.bin.gz";
const MARKETS: [&str; 2] = [
    "AuvLduWd2g2kmD4kLTGH4c3jf7XqfA9qBUDb4Lmps7kM",
    "CNEcZ2Cf6bqQTb8r9BSyZQb8o47Qa6Fd72Z1dj5BDGqp",
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
async fn live_depth_incident_quotes_and_executes_without_mutating_accounts() {
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
        let data_key = key(&cache.0[&program].as_ref().unwrap().data, 4).unwrap();
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(&cache.0[&data_key].as_ref().unwrap().data[45..])
            ),
            hash
        );
    }
    let mut executed = 0;
    let mut worst_bps = 0f64;
    let mut worst_regular_bps = 0f64;
    let mut worst = None;
    for market in 0..2 {
        let v = load(&cache, market).await;
        assert_eq!(v.state().unwrap().clock.slot, 471112973);
        for mint in [true, false] {
            let (lo, hi) = v.bounds(u8::from(!mint), u8::from(mint)).unwrap();
            let mut amounts: Vec<_> = (0..=60).map(|k| lo + (hi - lo) * k / 60).collect();
            if mint {
                for amount in [
                    1_000_000,
                    10_000_000,
                    100_000_000,
                    500_000_000,
                    1_000_000_000,
                ] {
                    assert!((lo..=hi).contains(&amount));
                    amounts.push(amount);
                }
            }
            for amount in amounts {
                let r = request(&v, mint, amount);
                let q = v.quote(r.clone()).unwrap();
                let ix = v.build_with_minimum(&r, USER, q.expected_output).unwrap();
                let e = v
                    .execution_oracle()
                    .unwrap()
                    .execute(ix, r.input_mint, r.output_mint, amount)
                    .unwrap_or_else(|e| panic!("market={market} mint={mint} amount={amount}: {e}"));
                assert!(q.expected_output <= e.output);
                let bps = (e.output - q.expected_output) as f64 * 10_000.0 / e.output as f64;
                if bps > worst_bps {
                    worst = Some((market, mint, amount, q.expected_output, e.output));
                }
                worst_bps = worst_bps.max(bps);
                if amount >= 1_000_000 {
                    worst_regular_bps = worst_regular_bps.max(bps);
                }
                executed += 1;
            }
        }
    }
    assert_eq!(executed, 254);
    eprintln!(
        "original live capture: {executed} executions, zero failures/overquotes, worst underquote {worst_bps:.6} bp at {worst:?}; input >= one token {worst_regular_bps:.6} bp"
    );
}

#[tokio::test]
async fn other_markets_do_not_gate_declared_sol_depth() {
    let original = support::Snapshot::read(Path::new(CAPTURE));
    for market in 0..2 {
        let baseline = load(&original, market).await;
        let mut cache = support::Snapshot(original.0.clone());
        let asset = baseline.state().unwrap().config.asset;
        let d = &mut cache
            .0
            .get_mut(&ocelot::PERP_MAP)
            .unwrap()
            .as_mut()
            .unwrap()
            .data;
        let count = u32::from_le_bytes(d[32..36].try_into().unwrap()) as usize;
        let mut changed = 0;
        for i in 0..count {
            let o = 64 + i * 1584;
            let id =
                u16_at(d, o + 944).unwrap() as u64 | ((u16_at(d, o + 948).unwrap() as u64) << 16);
            if d[o + 1450] != 0 || d[o + 1567] != 0 || id == asset {
                continue;
            }
            d[o + 16..o + 24].fill(0); // Stale other-market mark.
            d[o + 811] = 10; // Different oracle policy.
            d[o + 812] = 10;
            d[o + 1456] = 5; // Special other-market pricing.
            changed += 1;
        }
        assert!(changed > 0);
        let v = load(&cache, market).await;
        for mint in [true, false] {
            let (lo, hi) = baseline.bounds(u8::from(!mint), u8::from(mint)).unwrap();
            assert_eq!(v.bounds(u8::from(!mint), u8::from(mint)).unwrap(), (lo, hi));
            for amount in [lo, lo + (hi - lo) / 2, hi] {
                let r = request(&v, mint, amount);
                let before = baseline.quote(r.clone()).unwrap();
                let after = v.quote(r).unwrap();
                assert_eq!(after.expected_output, before.expected_output);
                assert_eq!(after.price, before.price);
            }
        }
        // The traded asset still needs a supported policy and a fresh mark.
        let d = &mut cache
            .0
            .get_mut(&ocelot::PERP_MAP)
            .unwrap()
            .as_mut()
            .unwrap()
            .data;
        let o = (0..count)
            .map(|i| 64 + i * 1584)
            .find(|&o| {
                d[o + 1450] == 0
                    && d[o + 1567] == 0
                    && (u16_at(d, o + 944).unwrap() as u64
                        | ((u16_at(d, o + 948).unwrap() as u64) << 16))
                        == asset
            })
            .unwrap();
        assert!(u64_at(d, o + 16).unwrap() > 0);
        d[o + 811] = 10;
        let mut v = baseline;
        assert!(v.update_state(&cache).await.is_err());
        assert!(!v.initialized());
    }
}

#[tokio::test]
async fn simulation_must_check_output_when_maker_collateral_changes() {
    let original = support::Snapshot::read(Path::new(CAPTURE));
    for market in 0..2 {
        let baseline = load(&original, market).await;
        let mut cache = support::Snapshot(original.0.clone());
        let index = baseline.state().unwrap().exchange.indices[0];
        let d = &mut cache.0.get_mut(&index).unwrap().as_mut().unwrap().data;
        let maker = "DUGirckBgoaW3zoEPhTVVo68pZpXrTKuJrsLBLWcZQo2"
            .parse()
            .unwrap();
        let node = d[96..]
            .chunks_exact_mut(64)
            .find(|node| key(node, 16).unwrap() == maker)
            .unwrap();
        // Synthetic negative control: keep all book prices/sizes unchanged but
        // make the sole live spline maker deeply insolvent ($1B deficit).
        node[48..56].copy_from_slice(&(-1_000_000_000_000_000i64).to_le_bytes());
        let v = load(&cache, market).await;
        for mint in [true, false] {
            let r = request(&v, mint, 1_000_000);
            let q = v.quote(r.clone()).unwrap();
            assert_eq!(
                q.expected_output,
                baseline.quote(r.clone()).unwrap().expected_output
            );
            // A risk-increasing fill may use other, more expensive liquidity.
            // Therefore a successful zero-minimum simulation alone is NOT enough.
            let actual = v.reference_quote(&r).unwrap().output;
            let shortfall_expected = (market == 0 && mint) || (market == 1 && !mint);
            assert_eq!(actual < q.expected_output, shortfall_expected);
            let tight = q.expected_output * 9_950 / 10_000; // 0.5% tolerance
            let loose = q.expected_output * 9_800 / 10_000; // 2% tolerance
            for minimum in [q.expected_output, tight, loose] {
                let ix = v.build_with_minimum(&r, USER, minimum).unwrap();
                let result = v.execution_oracle().unwrap().execute(
                    ix,
                    r.input_mint,
                    r.output_mint,
                    r.amount,
                );
                if shortfall_expected && minimum != loose {
                    assert!(minimum > actual);
                    let error = result.unwrap_err().to_string();
                    assert!(error.contains("MinimumOutput"), "{error}");
                } else {
                    assert_eq!(result.unwrap().output, actual);
                }
            }
            eprintln!(
                "maker solvency control market={market} mint={mint} quote={} actual={actual} shortfall={shortfall_expected}",
                q.expected_output
            );
        }
    }
}
