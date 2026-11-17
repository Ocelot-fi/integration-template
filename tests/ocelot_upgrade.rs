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
        let cache = support::Snapshot::candidate(&path);
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
                        let state = v.state().unwrap();
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

#[tokio::test]
async fn regular_finance_price_policy_is_used_by_both_instructions() {
    for address in [
        "AuvLduWd2g2kmD4kLTGH4c3jf7XqfA9qBUDb4Lmps7kM",
        "CNEcZ2Cf6bqQTb8r9BSyZQb8o47Qa6Fd72Z1dj5BDGqp",
    ] {
        let key = address.parse().unwrap();
        let mut cache =
            support::Snapshot::candidate(Path::new("tests/fixtures/sol-normal-470602378.bin.gz"));
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
