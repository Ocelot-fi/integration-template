//! Book-sized IOC limits: current-state parity and delayed-mark execution.
#![allow(clippy::result_large_err)]
#[allow(dead_code)]
mod support;
use std::path::Path;
use titan_integration_template::{
    ocelot::{
        self, OcelotVenue,
        quote::{self, PhoenixQuoter},
    },
    trading_venue::{FromAccount, TradingVenue},
};
#[cfg(feature = "reference-simulation")]
use titan_integration_template::{
    ocelot::{USDC, codec::u64_at},
    trading_venue::{QuoteRequest, SwapType},
};
const MARKETS: [&str; 2] = [
    "AuvLduWd2g2kmD4kLTGH4c3jf7XqfA9qBUDb4Lmps7kM",
    "CNEcZ2Cf6bqQTb8r9BSyZQb8o47Qa6Fd72Z1dj5BDGqp",
];
async fn load(file: &str, market: usize) -> (OcelotVenue, support::Snapshot) {
    let cache =
        support::Snapshot::candidate(&Path::new("tests/fixtures").join(format!("{file}.bin.gz")));
    let key = MARKETS[market].parse().unwrap();
    let mut v = OcelotVenue::from_account(&key, cache.0[&key].as_ref().unwrap()).unwrap();
    v.update_state(&cache).await.unwrap();
    (v, cache)
}
#[cfg(feature = "reference-simulation")]
fn request(v: &OcelotVenue, mint: bool, amount: u64) -> QuoteRequest {
    let lt = v.state().unwrap().market.mint;
    QuoteRequest {
        input_mint: if mint { USDC } else { lt },
        output_mint: if mint { lt } else { USDC },
        amount,
        swap_type: SwapType::ExactIn,
    }
}
#[cfg(feature = "reference-simulation")]
fn price_offset(mint: bool) -> usize {
    if mint { 59 } else { 23 }
}

#[tokio::test]
async fn book_cushion_is_capped_and_shared_by_quotes_and_instructions() {
    let (v, _) = load("sol-normal-470602378", 0).await;
    let mut s = v.state().unwrap().clone();
    let provider = ocelot::phoenix::NativePhoenix::prepare(&s).unwrap();
    let mut order = quote::order(&s, 10_000_000, true).unwrap();
    // One-lot quote exposes the required level independently of limit selection.
    order.lots = 1;
    for buy in [true, false] {
        order.buy = buy;
        order.reduce_only = !buy;
        if !buy {
            order.deposit = 0;
        }
        order.limit_ticks = if buy { s.mark.ticks * 2 } else { 1 };
        let fill = provider.quote(&order).unwrap();
        let worst = fill.quote_atoms / s.config.tick_size;
        let padded = if buy {
            (worst * 10_025).div_ceil(10_000)
        } else {
            worst * 9_975 / 10_000
        };
        assert_eq!(provider.execution_limit(&order).unwrap(), padded);
        order.limit_ticks = worst;
        assert_eq!(
            provider.execution_limit(&order).unwrap(),
            worst,
            "clip cushion at cap, never exclude required fill"
        );
        order.limit_ticks = if buy { worst - 1 } else { worst + 1 };
        assert!(provider.execution_limit(&order).is_err());
    }
    // Includes low-priced assets and policies smaller than the reserve.
    for mark in [1, 99, 7237, 10_000, u64::MAX / 2] {
        s.mark.ticks = mark;
        for policy in [1u16, 5, 10, 11, 50, 100, 200] {
            s.price_bps = policy;
            for mint in [true, false] {
                let order = quote::order(&s, 10_000_000, mint).unwrap();
                let bps = policy.saturating_sub(10).min(100) as u128;
                let expected = if order.buy {
                    mark as u128 * (10_000 + bps) / 10_000
                } else {
                    (mark as u128 * (10_000 - bps)).div_ceil(10_000)
                };
                assert_eq!(order.limit_ticks as u128, expected);
                assert!(order.limit_ticks > 0);
            }
        }
    }
}

#[cfg(feature = "reference-simulation")]
#[tokio::test]
async fn generated_book_limits_execute_across_normal_locked_and_crossed_ranges() {
    use ocelot::simulation::USER;
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
    ] {
        let (mut v, cache) = load(file, market).await;
        v.load_reference_oracle(&cache).await.unwrap();
        for mint in [true, false] {
            let (lo, hi) = v.bounds(u8::from(!mint), u8::from(mint)).unwrap();
            for k in 0..=20 {
                let amount = lo + (hi - lo) * k / 20;
                let r = request(&v, mint, amount);
                let q = v.quote(r.clone()).unwrap();
                let ix = v.build_with_minimum(&r, USER, q.expected_output).unwrap();
                let cap = quote::order(v.state().unwrap(), amount, mint).unwrap();
                let limit = u64_at(&ix.data, price_offset(mint)).unwrap();
                assert!(if cap.buy {
                    limit <= cap.limit_ticks
                } else {
                    limit >= cap.limit_ticks
                });
                let e = v
                    .execution_oracle()
                    .unwrap()
                    .execute(ix, r.input_mint, r.output_mint, amount)
                    .unwrap_or_else(|e| panic!("{file}/{market}/{mint}/{amount}: {e}"));
                let reference = v.reference_quote(&r).unwrap();
                assert_eq!(
                    e.output, reference.output,
                    "tightening must not change captured-state fills"
                );
                assert!(e.output >= q.expected_output);
                executed += 1;
            }
        }
    }
    assert_eq!(executed, 420);
    println!("IOC generated instructions: {executed} executions, unchanged fills, zero overquotes");
}

#[cfg(feature = "reference-simulation")]
#[tokio::test]
async fn signed_book_limit_survives_small_mark_changes_that_rejected_the_old_limit() {
    use ocelot::{codec::u16_at, simulation::USER};
    let (mut executed, mut old_rejected) = (0, 0);
    for market in 0..2 {
        let (mut v, cache) = load("sol-normal-470602378", market).await;
        v.load_reference_oracle(&cache).await.unwrap();
        let s = v.state().unwrap();
        for mint in [true, false] {
            let (lo, hi) = v.bounds(u8::from(!mint), u8::from(mint)).unwrap();
            // Mint quantity is signed, unlike V4 redemption. Isolate IOC from
            // changes in mint lot sizing by using a small non-boundary mint.
            // Full current-state mint ranges are executed in the test above.
            let amounts = if mint {
                vec![10_000_000]
            } else {
                vec![lo.saturating_mul(10).min(hi), lo + (hi - lo) / 2, hi]
            };
            for amount in amounts {
                let r = request(&v, mint, amount);
                v.quote(r.clone()).unwrap();
                // Floor=1 isolates the IOC guard from output movement. This is
                // not a promise that an old quote survives every user slippage.
                let ix = v.build_with_minimum(&r, USER, 1).unwrap();
                let order = quote::order(s, amount, mint).unwrap();
                let old_limit = if order.buy {
                    s.mark.ticks * 10_050 / 10_000
                } else {
                    (s.mark.ticks * 9_950).div_ceil(10_000)
                };
                for offset in [-5i64, -1, 0, 1, 5] {
                    let mut perp = s.accounts[&ocelot::PERP_MAP].clone();
                    let d = &mut perp.data;
                    let o = (64..d.len().saturating_sub(1583))
                        .step_by(1584)
                        .find(|&o| {
                            u16_at(d, o + 944).unwrap() as u64
                                | ((u16_at(d, o + 948).unwrap() as u64) << 16)
                                == s.config.asset
                        })
                        .unwrap();
                    let mark = (s.mark.ticks as i64 + offset) as u64;
                    d[o + 24..o + 32].copy_from_slice(&mark.to_le_bytes());
                    d[o + 136..o + 144].copy_from_slice(&0i64.to_le_bytes());
                    for j in 0..5 {
                        for off in [40, 176] {
                            let at = o + off + j * 16;
                            if u64_at(d, at).unwrap() > 0 {
                                d[at..at + 8].copy_from_slice(&mark.to_le_bytes());
                            }
                        }
                    }
                    let bank = v
                        .execution_oracle()
                        .unwrap()
                        .with_account(ocelot::PERP_MAP, perp)
                        .unwrap();
                    bank.execute(ix.clone(), r.input_mint, r.output_mint, amount)
                        .unwrap_or_else(|e| {
                            panic!(
                                "market={market} mint={mint} amount={amount} offset={offset}: {e}"
                            )
                        });
                    executed += 1;
                    if (order.buy && offset < 0) || (!order.buy && offset > 0) {
                        let mut old = ix.clone();
                        let at = price_offset(mint);
                        old.data[at..at + 8].copy_from_slice(&old_limit.to_le_bytes());
                        let error = bank
                            .execute(old, r.input_mint, r.output_mint, amount)
                            .unwrap_err();
                        assert!(
                            error.to_string().contains("FinanceExecutionBound"),
                            "{error}"
                        );
                        old_rejected += 1;
                    }
                }
            }
        }
    }
    assert_eq!((executed, old_rejected), (40, 16));
    println!(
        "IOC changed-mark executions={executed}; old limit failures reproduced={old_rejected}"
    );
}
