#![cfg(feature = "reference-simulation")]
#[allow(dead_code)]
mod support;
use std::path::Path;
use titan_integration_template::{
    ocelot::{self, OcelotVenue, USDC, simulation::USER},
    trading_venue::{FromAccount, QuoteRequest, SwapType, TradingVenue},
};

#[tokio::test]
async fn upgraded_program_executes_current_quotes() {
    let mut compared = 0;
    for (file, address) in [
        (
            "sol3l-465928618",
            "AuvLduWd2g2kmD4kLTGH4c3jf7XqfA9qBUDb4Lmps7kM",
        ),
        (
            "sol2s-465928707",
            "CNEcZ2Cf6bqQTb8r9BSyZQb8o47Qa6Fd72Z1dj5BDGqp",
        ),
        (
            "sol3l-467430913",
            "AuvLduWd2g2kmD4kLTGH4c3jf7XqfA9qBUDb4Lmps7kM",
        ),
        (
            "sol2s-467431351",
            "CNEcZ2Cf6bqQTb8r9BSyZQb8o47Qa6Fd72Z1dj5BDGqp",
        ),
        (
            "sol-normal-470602378",
            "AuvLduWd2g2kmD4kLTGH4c3jf7XqfA9qBUDb4Lmps7kM",
        ),
        (
            "sol-normal-470602378",
            "CNEcZ2Cf6bqQTb8r9BSyZQb8o47Qa6Fd72Z1dj5BDGqp",
        ),
    ] {
        let path = Path::new("tests/fixtures").join(format!("{file}.bin.gz"));
        let cache = if file == "sol-normal-470602378" {
            support::Snapshot::read(&path) // unmodified post-upgrade capture
        } else {
            support::Snapshot::current(&path) // historical state, current executable
        };
        let key = address.parse().unwrap();
        let mut v = OcelotVenue::from_account(&key, cache.0[&key].as_ref().unwrap()).unwrap();
        v.update_state(&cache).await.unwrap();
        v.load_reference_oracle(&cache).await.unwrap();
        let lt = v.state().unwrap().market.mint;
        for mint in [true, false] {
            let mut count = 0;
            eprintln!(
                "UPGRADE_BOUNDS {file} mint={mint} {:?}",
                v.bounds(u8::from(!mint), u8::from(mint))
            );
            for amount in [1_000_000, 10_000_000, 100_000_000, 1_000_000_000] {
                let r = QuoteRequest {
                    input_mint: if mint { USDC } else { lt },
                    output_mint: if mint { lt } else { USDC },
                    amount,
                    swap_type: SwapType::ExactIn,
                };
                match v.quote(r.clone()) {
                    Err(e) => eprintln!("{file} mint={mint} input={amount} DECLINE {e}"),
                    Ok(q) => {
                        let ix = v.build_with_minimum(&r, USER, 1).unwrap();
                        assert_eq!(
                            &ix.data[..8],
                            &ocelot::discriminator(if mint {
                                "global:phoenix_mint_v3"
                            } else {
                                "global:phoenix_redeem_v4"
                            })
                        );
                        let (result, post) = v
                            .execution_oracle()
                            .unwrap()
                            .execute_with_accounts(ix, r.input_mint, r.output_mint, amount)
                            .unwrap_or_else(|e| panic!("{file} mint={mint} input={amount}: {e}"));
                        let pm = ocelot::codec::mark(
                            &post[&ocelot::PERP_MAP],
                            &v.state().unwrap().config,
                            v.state().unwrap().clock.slot,
                        )
                        .unwrap();
                        let state = v.state().unwrap();
                        assert!(
                            (state.live_mark_bounds.0..=state.live_mark_bounds.1)
                                .contains(&pm.quote_per_lot),
                            "post-trade mark outside prepared hull"
                        );
                        let after =
                            ocelot::codec::position(&post[&state.config.trader], &state.config)
                                .unwrap();
                        let order = ocelot::quote::order(state, amount, mint).unwrap();
                        let expected_base = state.position.base as i128
                            + if order.buy {
                                order.lots as i128
                            } else {
                                -(order.lots as i128)
                            };
                        assert_eq!(
                            after.base as i128, expected_base,
                            "V4 native close must match actual planner"
                        );
                        if !mint {
                            let mut retired = v.build_with_minimum(&r, USER, 1).unwrap();
                            retired.data[..8].copy_from_slice(&ocelot::discriminator(
                                "global:phoenix_redeem_v3",
                            ));
                            let error = v
                                .execution_oracle()
                                .unwrap()
                                .execute(retired, r.input_mint, r.output_mint, amount)
                                .unwrap_err()
                                .to_string();
                            assert!(error.contains("RedemptionV3Retired"), "{error}");
                            // V4 signs the side/price but reconstructs the close quantity.
                            let mut stale_quantity =
                                v.build_with_minimum(&r, USER, q.expected_output).unwrap();
                            stale_quantity.data[31..39].copy_from_slice(&u64::MAX.to_le_bytes());
                            stale_quantity.data[40..48].copy_from_slice(&u64::MAX.to_le_bytes());
                            assert_eq!(
                                v.execution_oracle()
                                    .unwrap()
                                    .execute(stale_quantity, r.input_mint, r.output_mint, amount)
                                    .unwrap()
                                    .output,
                                result.output
                            );
                        }
                        assert!(
                            q.expected_output <= result.output,
                            "{file} mint={mint} input={amount} quote={} actual={}",
                            q.expected_output,
                            result.output
                        );
                        count += 1;
                        compared += 1;
                        eprintln!(
                            "{file} mint={mint} input={amount} quote={} actual={}",
                            q.expected_output, result.output
                        );
                    }
                }
            }
            assert!(count > 0, "no accepted quotes {file} mint={mint}");
        }
    }
    eprintln!("UPGRADE comparisons={compared}");
}

/// The public SDK exposes component layout but not the mark evaluator. Exercise
/// the supported oracle-anchored profile against the pinned Phoenix executable;
/// this is evidence for that profile, not proof of arbitrary pricing policies.
#[tokio::test]
async fn post_trade_mark_hull_covers_oracle_weight_and_ema_variations() {
    use ocelot::codec::{position, u16_at, u64_at};
    let original = support::Snapshot::read(Path::new("tests/fixtures/sol-normal-470602378.bin.gz"));
    let mut comparisons = 0;
    for address in [
        "AuvLduWd2g2kmD4kLTGH4c3jf7XqfA9qBUDb4Lmps7kM",
        "CNEcZ2Cf6bqQTb8r9BSyZQb8o47Qa6Fd72Z1dj5BDGqp",
    ] {
        let key = address.parse().unwrap();
        let mut base = OcelotVenue::from_account(&key, original.0[&key].as_ref().unwrap()).unwrap();
        base.update_state(&original).await.unwrap();
        let asset = base.state().unwrap().config.asset;
        let d = &original.0[&ocelot::PERP_MAP].as_ref().unwrap().data;
        let o = (64..d.len().saturating_sub(1583))
            .step_by(1584)
            .find(|&o| {
                u16_at(d, o + 944).unwrap() as u64 | ((u16_at(d, o + 948).unwrap() as u64) << 16)
                    == asset
            })
            .unwrap();
        for weights in [
            [100u64, 100, 100],
            [0, 100, 0],
            [100, 0, 0],
            [300, 100, 200],
        ] {
            for ema in [-20i64, -1, 0, 1, 20] {
                let mut cache = support::Snapshot(original.0.clone());
                let d = &mut cache
                    .0
                    .get_mut(&ocelot::PERP_MAP)
                    .unwrap()
                    .as_mut()
                    .unwrap()
                    .data;
                for (offset, value) in [112, 248, 280].into_iter().zip(weights) {
                    d[o + offset..o + offset + 8].copy_from_slice(&value.to_le_bytes());
                }
                d[o + 136..o + 144].copy_from_slice(&ema.to_le_bytes());
                // Preserve component timestamps, response eligibility, and all
                // captured policy bytes; vary their prices inside a small hull.
                for j in 0..5 {
                    for offset in [40, 176] {
                        let start = o + offset + j * 16;
                        let p = u64_at(d, start).unwrap();
                        if p > 0 {
                            let moved = (p as i64 + j as i64 - 2) as u64;
                            d[start..start + 8].copy_from_slice(&moved.to_le_bytes());
                        }
                    }
                }
                let mut v = base.clone();
                v.update_state(&cache).await.unwrap();
                v.load_reference_oracle(&cache).await.unwrap();
                for mint in [true, false] {
                    let (lo, hi) = v.bounds(u8::from(!mint), u8::from(mint)).unwrap();
                    for amount in [lo, lo + (hi - lo) / 2, hi] {
                        let state = v.state().unwrap();
                        let r = QuoteRequest {
                            input_mint: if mint { USDC } else { state.market.mint },
                            output_mint: if mint { state.market.mint } else { USDC },
                            amount,
                            swap_type: SwapType::ExactIn,
                        };
                        let q = v.quote(r.clone()).unwrap();
                        let ix = v.build_with_minimum(&r, USER, q.expected_output).unwrap();
                        let (e, post) = v
                            .execution_oracle()
                            .unwrap()
                            .execute_with_accounts(ix, r.input_mint, r.output_mint, amount)
                            .unwrap_or_else(|e| {
                                panic!(
                                    "weights={weights:?} ema={ema} mint={mint} amount={amount}: {e}"
                                )
                            });
                        assert!(q.expected_output <= e.output);
                        let mark = ocelot::codec::mark(
                            &post[&ocelot::PERP_MAP],
                            &state.config,
                            state.clock.slot,
                        )
                        .unwrap();
                        assert!(
                            (state.live_mark_bounds.0..=state.live_mark_bounds.1)
                                .contains(&mark.quote_per_lot),
                            "weights={weights:?} ema={ema} post-mark={}",
                            mark.ticks
                        );
                        let after = position(&post[&state.config.trader], &state.config).unwrap();
                        let order = ocelot::quote::order(state, amount, mint).unwrap();
                        assert_eq!(
                            after.base as i128,
                            state.position.base as i128
                                + if order.buy {
                                    order.lots as i128
                                } else {
                                    -(order.lots as i128)
                                }
                        );
                        comparisons += 1;
                    }
                }
            }
        }
        let mut unsupported = support::Snapshot(original.0.clone());
        let d = &mut unsupported
            .0
            .get_mut(&ocelot::PERP_MAP)
            .unwrap()
            .as_mut()
            .unwrap()
            .data;
        d[o + 280..o + 288].copy_from_slice(&200u64.to_le_bytes());
        assert!(
            base.update_state(&unsupported)
                .await
                .unwrap_err()
                .to_string()
                .contains("book-dominant")
        );
        assert!(!base.initialized());
    }
    assert_eq!(comparisons, 240);
    println!("post-trade oracle hull: {comparisons} accepted executions");
}

#[tokio::test]
async fn regular_finance_price_policy_is_used_by_both_instructions() {
    for address in [
        "AuvLduWd2g2kmD4kLTGH4c3jf7XqfA9qBUDb4Lmps7kM",
        "CNEcZ2Cf6bqQTb8r9BSyZQb8o47Qa6Fd72Z1dj5BDGqp",
    ] {
        let key = address.parse().unwrap();
        let mut cache =
            support::Snapshot::read(Path::new("tests/fixtures/sol-normal-470602378.bin.gz"));
        let finance = ocelot::instruction::pda(b"finance_v1", key);
        cache.0.get_mut(&finance).unwrap().as_mut().unwrap().data[59..61]
            .copy_from_slice(&25u16.to_le_bytes());
        let mut v = OcelotVenue::from_account(&key, cache.0[&key].as_ref().unwrap()).unwrap();
        v.update_state(&cache).await.unwrap();
        v.load_reference_oracle(&cache).await.unwrap();
        for mint in [true, false] {
            let state = v.state().unwrap();
            let order = ocelot::quote::order(state, 1_000_000, mint).unwrap();
            let limit = if order.buy {
                state.mark.ticks as u128 * 10_025 / 10_000
            } else {
                (state.mark.ticks as u128 * 9_975).div_ceil(10_000)
            };
            assert_eq!(order.limit_ticks as u128, limit);
            let r = QuoteRequest {
                input_mint: if mint { USDC } else { state.market.mint },
                output_mint: if mint { state.market.mint } else { USDC },
                amount: 1_000_000,
                swap_type: SwapType::ExactIn,
            };
            let q = v.quote(r.clone()).unwrap();
            assert!(q.expected_output <= v.reference_quote(&r).unwrap().output);
        }
        for invalid in [0u16, 1000] {
            cache.0.get_mut(&finance).unwrap().as_mut().unwrap().data[59..61]
                .copy_from_slice(&invalid.to_le_bytes());
            assert!(v.update_state(&cache).await.is_err());
            assert!(!v.initialized());
        }
    }
}
