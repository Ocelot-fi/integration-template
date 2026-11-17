#![cfg(feature = "reference-simulation")]
#[allow(dead_code)]
mod support;
use std::path::Path;
use titan_integration_template::{
    ocelot::{
        self, OcelotVenue, USDC,
        codec::{u16_at, u64_at},
        simulation::USER,
    },
    trading_venue::{FromAccount, QuoteRequest, SwapType, TradingVenue},
};
const MARKETS: [&str; 2] = [
    "AuvLduWd2g2kmD4kLTGH4c3jf7XqfA9qBUDb4Lmps7kM",
    "CNEcZ2Cf6bqQTb8r9BSyZQb8o47Qa6Fd72Z1dj5BDGqp",
];
async fn load(cache: &support::Snapshot, key: solana_pubkey::Pubkey) -> OcelotVenue {
    let mut v = OcelotVenue::from_account(&key, cache.0[&key].as_ref().unwrap()).unwrap();
    v.update_state(cache).await.unwrap();
    v.load_reference_oracle(cache).await.unwrap();
    v
}

/// Same captured accounts and unchanged Phoenix/Ember executables on both sides.
/// The deployed ELF reproduces the reviewer failure; the candidate must execute
/// every sampled redemption, close the proportional exposure and never overquote.
#[tokio::test]
async fn proportional_redemptions_survive_execution_time_mark_updates() {
    let path = Path::new("tests/fixtures/sol-normal-470602378.bin.gz");
    let mut candidate_count = 0;
    for (market, address) in MARKETS.iter().enumerate() {
        let key = address.parse().unwrap();
        for one_perp in [true, false] {
            for offset in [-3i64, -1, 0, 1, 3] {
                if one_perp && offset != -1 {
                    continue;
                }
                for candidate in [false, true] {
                    let mut cache = if candidate {
                        support::Snapshot::candidate(path)
                    } else {
                        support::Snapshot::read(path)
                    };
                    let v = load(&cache, key).await;
                    let asset = v.state().unwrap().config.asset;
                    let d = &mut cache
                        .0
                        .get_mut(&ocelot::PERP_MAP)
                        .unwrap()
                        .as_mut()
                        .unwrap()
                        .data;
                    let o = (64..d.len().saturating_sub(1583))
                        .step_by(1584)
                        .find(|&o| {
                            u16_at(d, o + 944).unwrap() as u64
                                | ((u16_at(d, o + 948).unwrap() as u64) << 16)
                                == asset
                        })
                        .unwrap();
                    let ticks = (u64_at(d, o + 24).unwrap() as i64 + offset) as u64;
                    if one_perp {
                        // One existing observation changes; the captured EMA is preserved.
                        d[o + 176..o + 184].copy_from_slice(&ticks.to_le_bytes());
                    } else {
                        d[o + 136..o + 144].copy_from_slice(&0i64.to_le_bytes());
                        for j in 0..5 {
                            for off in [40, 176] {
                                let at = o + off + j * 16;
                                if u64_at(d, at).unwrap() > 0 {
                                    d[at..at + 8].copy_from_slice(&ticks.to_le_bytes());
                                }
                            }
                        }
                    }
                    let v = load(&cache, key).await;
                    let s = v.state().unwrap();
                    let (lo, hi) = v.bounds(1, 0).unwrap();
                    assert!(
                        s.actual_supply - hi <= 100,
                        "must test the full redemption range"
                    );
                    let mut failed = 0;
                    for k in 0..=60 {
                        let amount = lo + (hi - lo) / 60 * k;
                        let r = QuoteRequest {
                            input_mint: s.market.mint,
                            output_mint: USDC,
                            amount,
                            swap_type: SwapType::ExactIn,
                        };
                        let q = v.quote(r.clone()).unwrap();
                        let ix = v
                            .build_with_minimum(
                                &r,
                                USER,
                                if candidate { q.expected_output } else { 1 },
                            )
                            .unwrap();
                        match v.execution_oracle().unwrap().execute_with_accounts(
                            ix,
                            r.input_mint,
                            r.output_mint,
                            amount,
                        ) {
                            Ok((e, post)) => {
                                if candidate {
                                    assert!(
                                        q.expected_output <= e.output,
                                        "market={market} offset={offset} amount={amount}"
                                    );
                                    let after =
                                        ocelot::codec::position(&post[&s.config.trader], &s.config)
                                            .unwrap();
                                    let before = s.position.base.unsigned_abs();
                                    let closed = before - after.base.unsigned_abs();
                                    assert!(
                                        closed as u128 * s.market.shares as u128
                                            >= before as u128 * amount as u128
                                    );
                                    assert_eq!(
                                        closed,
                                        ocelot::quote::order(s, amount, false).unwrap().lots
                                    );
                                    candidate_count += 1;
                                }
                            }
                            Err(e) => {
                                assert!(
                                    !candidate,
                                    "candidate market={market} one_perp={one_perp} offset={offset} amount={amount}: {e}"
                                );
                                assert!(e.to_string().contains("FinanceExecutionBound"), "{e}");
                                failed += 1;
                            }
                        }
                    }
                    if !candidate && market == 0 && offset == -1 {
                        assert_eq!(failed, 46, "must reproduce the original defect");
                    }
                    println!(
                        "PROPORTIONAL market={market} one_perp={one_perp} offset={offset} candidate={candidate} executed={} failed={failed}",
                        61 - failed
                    );
                }
            }
        }
    }
    assert_eq!(candidate_count, 732);
}
