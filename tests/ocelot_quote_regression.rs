#![allow(clippy::result_large_err)]
//! Frozen November 17 current-state quote/instruction baseline. Numerical changes
//! are independently exercised against the pinned candidate executable.
//! Previous V4 instruction hash: 43c34f24af5754ed44e0eb7dd177648199e628da80ecbed44a7e4ce00a2fa93b.
//! Redemption lot hints and three tiny-input error strings change; accepted quotes stay pinned.
//! Pre-book-cushion instruction hash: c1b75fc343bc525b6c7fdee85a9ee03478c0a4dd87387c05061a50731a512f95.
//! Book-cushioned IOC prices change instruction bytes; accepted quotes stay pinned.
//! Initial upgrade/mark-forecast hash: a7bec29e5ba5364724678e0bf0dbb56b4115f170ee96adba7f1d33e50d419cb5.
//! Pre-upgrade hash: 67eee7455bf965a2bfe591154a5ee41a0b531fda53c310d65ce1ee8fa0bafafe.
#[allow(dead_code)]
mod support;
use solana_pubkey::Pubkey;
use std::path::Path;
use titan_integration_template::{
    ocelot::{OcelotVenue, USDC},
    trading_venue::{FromAccount, QuoteRequest, SwapType, TradingVenue},
};
#[tokio::test]
async fn normal_quotes_and_instructions_match_upgrade_baseline() {
    let mut out = String::new();
    for (file, market) in [
        (
            "sol3l-465928618.bin.gz",
            "AuvLduWd2g2kmD4kLTGH4c3jf7XqfA9qBUDb4Lmps7kM",
        ),
        (
            "sol2s-465928707.bin.gz",
            "CNEcZ2Cf6bqQTb8r9BSyZQb8o47Qa6Fd72Z1dj5BDGqp",
        ),
        (
            "sol3l-467430913.bin.gz",
            "AuvLduWd2g2kmD4kLTGH4c3jf7XqfA9qBUDb4Lmps7kM",
        ),
        (
            "sol2s-467431351.bin.gz",
            "CNEcZ2Cf6bqQTb8r9BSyZQb8o47Qa6Fd72Z1dj5BDGqp",
        ),
    ] {
        let cache = support::Snapshot::candidate(&Path::new("tests/fixtures").join(file));
        let key: Pubkey = market.parse().unwrap();
        let mut v = OcelotVenue::from_account(&key, cache.0[&key].as_ref().unwrap()).unwrap();
        v.update_state(&cache).await.unwrap();
        let lt = v.state().unwrap().market.mint;
        let supply = v.state().unwrap().actual_supply;
        for mint in [true, false] {
            let b = v.bounds(u8::from(!mint), u8::from(mint));
            out += &format!("{file} mint={mint} bounds={b:?}\n");
            let (lo, hi) = b.unwrap_or((1, 2_000_000_000));
            let mut amounts: Vec<u64> = (0..4000u64).map(|i| lo + (hi - lo) / 3999 * i).collect();
            amounts.extend([
                0,
                1,
                lo.saturating_sub(1),
                hi + 1,
                supply,
                supply.saturating_sub(1),
            ]);
            let mut seed = 7u64;
            for _ in 0..4000 {
                seed = seed
                    .wrapping_mul(6364136223846793005)
                    .wrapping_add(1442695040888963407);
                amounts.push(seed % (hi.saturating_mul(2) + 1));
            }
            for a in amounts {
                let r = QuoteRequest {
                    input_mint: if mint { USDC } else { lt },
                    output_mint: if mint { lt } else { USDC },
                    amount: a,
                    swap_type: SwapType::ExactIn,
                };
                let q = v.quote(r.clone());
                let line = match q {
                    Ok(q) => format!("{a} ok {} {:e}", q.expected_output, q.price),
                    Err(e) => format!("{a} err {e}"),
                };
                out += &format!("{file} {mint} {line}\n");
                if a % 97 == 0 && a > 0 {
                    let ix = v.generate_swap_instruction(r, Pubkey::new_from_array([211; 32]));
                    out += &format!(
                        "  ix {:?}\n",
                        ix.map(|i| (
                            i.data,
                            i.accounts
                                .iter()
                                .map(|m| (m.pubkey, m.is_signer, m.is_writable))
                                .collect::<Vec<_>>()
                        ))
                    );
                }
            }
        }
    }
    println!("upgrade baseline: {} records", out.lines().count());
    use sha2::{Digest, Sha256};
    // Independently compared with the prior implementation: all accepted
    // outputs, marginal prices and bounds are identical. Three one-atom inputs
    // still decline but now report that closing costs consume the claim.
    let accepted = out
        .lines()
        .filter(|line| {
            line.contains(" bounds=") || (line.contains(" ok ") && !line.starts_with("  ix "))
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        format!("{:x}", Sha256::digest(accepted.as_bytes())),
        "50b34b5ffae92de2f8772a6358c6d03cfe6e376fa47c0cb23c3aed3fd2137288"
    );
    assert_eq!(
        format!("{:x}", Sha256::digest(out.as_bytes())),
        "32386193f752dea354edf1d5287d90154c588347a8559dc19caf4a32ce13fe4b"
    );
}
