#![cfg(feature = "reference-simulation")]
#[allow(dead_code)]
mod support;
use std::{collections::BTreeSet, path::Path};
use titan_integration_template::{
    ocelot::{self, OcelotVenue, USDC, codec::u64_at, simulation::USER},
    trading_venue::{FromAccount, QuoteRequest, SwapType, TradingVenue},
};
const LONG: &str = "AuvLduWd2g2kmD4kLTGH4c3jf7XqfA9qBUDb4Lmps7kM";
const SHORT: &str = "CNEcZ2Cf6bqQTb8r9BSyZQb8o47Qa6Fd72Z1dj5BDGqp";
fn request(v: &OcelotVenue, amount: u64, mint: bool) -> QuoteRequest {
    let token = v.state().unwrap().market.mint;
    QuoteRequest {
        input_mint: if mint { USDC } else { token },
        output_mint: if mint { token } else { USDC },
        amount,
        swap_type: SwapType::ExactIn,
    }
}
async fn load(cache: &support::Snapshot, key: solana_pubkey::Pubkey) -> OcelotVenue {
    let mut v = OcelotVenue::from_account(&key, cache.0[&key].as_ref().unwrap()).unwrap();
    v.update_state(cache).await.unwrap();
    v.load_reference_oracle(cache).await.unwrap();
    v
}

/// Sample the WHOLE redemption supply, including outside bounds. Report both
/// executable quotes we decline and accepted quotes which fail execution.
/// Neither class may disappear behind an outside-bounds `continue`.
#[tokio::test]
async fn redemption_coverage_and_execution_failures_are_measured() {
    let mut rejected = Vec::new();
    for (file, address) in [
        ("sol3l-465928618", LONG),
        ("sol2s-465928707", SHORT),
        ("sol3l-467430913", LONG),
        ("sol2s-467431351", SHORT),
        ("sol-normal-470602378", LONG),
        ("sol-normal-470602378", SHORT),
    ] {
        let path = Path::new("tests/fixtures").join(format!("{file}.bin.gz"));
        let cache = if file == "sol-normal-470602378" {
            support::Snapshot::read(&path)
        } else {
            support::Snapshot::current(&path)
        };
        let v = load(&cache, address.parse().unwrap()).await;
        let s = v.state().unwrap();
        let (lo, hi) = v.bounds(1, 0).unwrap();
        let supply = s.actual_supply;
        let mut amounts: BTreeSet<u64> = (1..=200u128)
            .map(|n| (n * supply as u128 / 200) as u64)
            .collect();
        amounts.extend([lo, hi, supply - 1, supply]);
        if file == "sol-normal-470602378" {
            amounts.extend(if address == SHORT {
                vec![
                    954197, 954198, 970153, 970154, 1_000_000, 2_000_000, 5_000_000,
                ]
            } else {
                vec![318027904, 318028004, 328420381, 1108418784, 2493942264]
            });
            assert!(
                supply - hi <= 100,
                "fresh routing range must reach full supply, not the old prefix"
            );
        }
        let (
            mut executed,
            mut declined_executable,
            mut failed_accepted,
            mut in_range,
            mut holes,
            mut outside_executable,
        ) = (0, 0, 0, 0, 0, 0);
        for amount in &amounts {
            let r = request(&v, *amount, false);
            let q = v.quote(r.clone());
            let e = v.reference_quote(&r);
            if (lo..=hi).contains(amount) {
                in_range += 1;
                if q.is_err() {
                    holes += 1;
                }
            }
            match (q, e) {
                (Ok(q), Ok(e)) => {
                    assert!(
                        q.expected_output <= e.output,
                        "{file} {address} {amount}: overquote"
                    );
                    executed += 1;
                    if !(lo..=hi).contains(amount) {
                        outside_executable += 1;
                    }
                }
                (Err(q), Ok(_)) => {
                    executed += 1;
                    declined_executable += 1;
                    println!("DECLINED_EXECUTABLE {file} {amount}: {q}");
                }
                (Ok(_), Err(e)) => {
                    failed_accepted += 1;
                    support::assert_execution_risk_rejection(&e);
                    println!("POST_TRADE_REJECTION {file} {amount}");
                    rejected.push((file, *amount));
                }
                (Err(_), Err(_)) => {}
            }
        }
        println!(
            "AVAILABILITY {}",
            serde_json::json!({"file":file,"market":address,"samples":amounts.len(),"supply":supply,"lo":lo,"hi":hi,"executed":executed,"declined_executable":declined_executable,"failed_accepted":failed_accepted,"in_range":in_range,"quote_holes":holes,"outside_executable":outside_executable})
        );
        assert_eq!(holes, 0, "cached-state quote range contains a hole");
        assert_eq!(
            declined_executable, 0,
            "availability regression in captured state"
        );
        assert_eq!(
            executed + failed_accepted,
            amounts.len(),
            "sample disappeared into a joint quote/execution rejection"
        );
    }
    // Pin every observed rejected input, rather than permitting arbitrary
    // failures or shrinking the routing domain to hide them.
    assert_eq!(
        rejected,
        vec![
            ("sol3l-467430913", 3_155_519_257),
            ("sol2s-467431351", 2_198_157),
            ("sol2s-467431351", 2_547_864),
            ("sol2s-467431351", 2_897_570),
            ("sol2s-467431351", 6_094_890),
            ("sol2s-467431351", 6_444_597),
        ]
    );
}

/// The host no longer forecasts the post-order reference price. Demonstrate
/// both the changing EMA and an actual post-order rejection, without accepting
/// wrong payouts or pretending every quote necessarily executes.
#[tokio::test]
async fn post_order_price_changes_are_left_to_simulation_and_program_guards() {
    let original = support::Snapshot::read(Path::new("tests/fixtures/sol-normal-470602378.bin.gz"));
    let key = LONG.parse().unwrap();
    let base = load(&original, key).await;
    let asset = base.state().unwrap().config.asset;
    let d = &original.0[&ocelot::PERP_MAP].as_ref().unwrap().data;
    let o = (64..d.len().saturating_sub(1583))
        .step_by(1584)
        .find(|&o| {
            ocelot::codec::u16_at(d, o + 944).unwrap() as u64
                | ((ocelot::codec::u16_at(d, o + 948).unwrap() as u64) << 16)
                == asset
        })
        .unwrap();
    for (oracle, stored_mark, amount, mint) in [
        (7454u64, 7454u64, 100_000_000u64, true),
        (7236, 7237, 1_108_418_784, false),
    ] {
        let mut cache = support::Snapshot(original.0.clone());
        let d = &mut cache
            .0
            .get_mut(&ocelot::PERP_MAP)
            .unwrap()
            .as_mut()
            .unwrap()
            .data;
        d[o + 24..o + 32].copy_from_slice(&stored_mark.to_le_bytes());
        d[o + 136..o + 144].copy_from_slice(&0i64.to_le_bytes());
        for j in 0..5 {
            for offset in [40, 176] {
                let start = o + offset + 16 * j;
                if u64_at(d, start).unwrap() > 0 {
                    d[start..start + 8].copy_from_slice(&oracle.to_le_bytes());
                }
            }
        }
        let v = load(&cache, key).await;
        let r = request(&v, amount, mint);
        let q = v
            .quote(r.clone())
            .expect("quote uses supplied mark, not forecast");
        let ix = v.build_with_minimum(&r, USER, q.expected_output).unwrap();
        let e = v.execution_oracle().unwrap().execute_with_accounts(
            ix,
            r.input_mint,
            r.output_mint,
            amount,
        );
        if mint {
            let (e, post) = e.unwrap();
            assert!(q.expected_output <= e.output);
            assert_eq!(u64_at(&post[&ocelot::PERP_MAP].data, o + 24).unwrap(), 7453);
            assert_eq!(
                ocelot::codec::i64_at(&post[&ocelot::PERP_MAP].data, o + 136).unwrap(),
                -1
            );
        } else {
            support::assert_execution_risk_rejection(&e.unwrap_err());
        }
    }
}
