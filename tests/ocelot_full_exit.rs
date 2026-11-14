#![cfg(feature = "reference-simulation")]
#![allow(clippy::result_large_err)]
//! Original-account executions of full-market exits, outside routed bounds.
#[allow(dead_code)]
mod support;
use solana_pubkey::Pubkey;
use std::path::Path;
use titan_integration_template::{
    ocelot::{
        self, OcelotVenue, USDC,
        codec::u64_at,
        phoenix::NativePhoenixFactory,
        quote::{Limits, PhoenixQuoteFactory},
        spline::SplineDepth,
    },
    trading_venue::{FromAccount, QuoteRequest, SwapType, TradingVenue},
};
const MARKETS: [&str; 2] = [
    "AuvLduWd2g2kmD4kLTGH4c3jf7XqfA9qBUDb4Lmps7kM",
    "CNEcZ2Cf6bqQTb8r9BSyZQb8o47Qa6Fd72Z1dj5BDGqp",
];
fn put64(d: &mut [u8], o: usize, n: u64) {
    d[o..o + 8].copy_from_slice(&n.to_le_bytes());
}
async fn decoded(c: &support::Snapshot, key: Pubkey) -> OcelotVenue {
    let mut v = OcelotVenue::from_account(&key, c.0[&key].as_ref().unwrap()).unwrap();
    v.update_state(c).await.unwrap();
    v
}
fn request(v: &OcelotVenue) -> QuoteRequest {
    let s = v.state().unwrap();
    QuoteRequest {
        input_mint: s.market.mint,
        output_mint: USDC,
        amount: s.actual_supply,
        swap_type: SwapType::ExactIn,
    }
}
async fn crossed(market: usize, shift: u64) -> (support::Snapshot, OcelotVenue) {
    let mut c = support::Snapshot::read(Path::new("tests/fixtures/sol-crossed-468615690.bin.gz"));
    let key = MARKETS[market].parse().unwrap();
    let v = decoded(&c, key).await;
    let s = v.state().unwrap();
    let k = Pubkey::find_program_address(&[b"spline", s.config.book.as_ref()], &ocelot::PHOENIX).0;
    let original = &s.accounts[&k];
    let n = u32::from_le_bytes(original.data[72..76].try_into().unwrap()) as usize;
    let mut best = (0, None);
    for i in 0..n {
        let mut isolated = original.clone();
        for j in 0..n {
            if i != j {
                isolated.data[112 + j * 1336 + 32] = 0;
            }
        }
        let depth = SplineDepth::from_account(&isolated, s.config.book, s.clock.slot).unwrap();
        let p = if s.position.base > 0 {
            depth.best_bid()
        } else {
            depth.best_ask().map(|x| u64::MAX - x)
        };
        if p > best.1 {
            best = (i, p);
        }
    }
    assert!(best.1.is_some());
    let d = &mut c.0.get_mut(&k).unwrap().as_mut().unwrap().data;
    let o = 112 + best.0 * 1336 + 40;
    let mid = u64_at(d, o).unwrap();
    put64(
        d,
        o,
        if s.position.base > 0 {
            mid + shift
        } else {
            mid - shift
        },
    );
    let v = decoded(&c, key).await;
    (c, v)
}

#[tokio::test]
async fn full_exit_quotes_keep_the_reviewed_curve_and_known_price_errors() {
    // Outputs independently recorded at 0581d19, before the capacity-only fix.
    let expected = [
        [
            2698236078, 2699237212, 2700238345, 2702240612, 2702344715, 2702344715,
        ],
        [
            209136425, 209189259, 209242091, 209347757, 209393624, 209393624,
        ],
    ];
    for (market, values) in expected.iter().enumerate() {
        for (shift, expected_output) in [0, 2, 4, 8, 16, 32].into_iter().zip(values) {
            let (cache, mut v) = crossed(market, shift).await;
            v.load_reference_oracle(&cache).await.unwrap();
            let r = request(&v);
            assert!(r.amount > v.bounds(1, 0).unwrap().1);
            let q = v.quote(r.clone()).unwrap();
            assert_eq!(q.expected_output, *expected_output);
            let e = v.reference_quote(&r).unwrap();
            if market == 0 && shift >= 8 {
                assert!(
                    q.expected_output > e.output,
                    "retained approximation limitation"
                );
            } else {
                assert!(q.expected_output <= e.output);
            }
            let s = v.state().unwrap();
            let f = NativePhoenixFactory
                .prepare(s)
                .unwrap()
                .quote(&ocelot::quote::order(s, r.amount, false).unwrap())
                .unwrap();
            // Invert rounded net output with ceil: this may overestimate actual
            // proceeds by fee-rounding dust, making this bound check stricter.
            let proceeds = (e.output as u128 * 10_000)
                .div_ceil(10_000 - s.market.redeem_fee_bps as u128)
                + s.liability as u128;
            let withdrawal = proceeds.saturating_sub(Limits::load(s).unwrap().cash as u128);
            assert!(f.collateral_upper_bound as u128 >= withdrawal);
        }
    }
}

#[tokio::test]
async fn full_exit_rejects_insufficient_capacity_without_changing_partial_quotes() {
    for (shift, old_cap, quoted) in [
        (8, 210_242_221, 209_347_757),
        (32, 210_878_221, 209_393_624),
    ] {
        let (original, v) = crossed(1, shift).await;
        let s = v.state().unwrap();
        let r = request(&v);
        let original_quote = v.quote(r.clone()).unwrap();
        assert_eq!(original_quote.expected_output, quoted);
        let f = NativePhoenixFactory
            .prepare(s)
            .unwrap()
            .quote(&ocelot::quote::order(s, r.amount, false).unwrap())
            .unwrap();
        let corrected_cap = f.collateral_upper_bound + Limits::load(s).unwrap().cash;
        assert!(corrected_cap > old_cap);
        for (cap, accepted) in [(old_cap, false), (corrected_cap, true)] {
            let mut cache = support::Snapshot(original.0.clone());
            let d = &mut cache
                .0
                .get_mut(&ocelot::global_bucket())
                .unwrap()
                .as_mut()
                .unwrap()
                .data;
            d[73] = 1;
            let limit = u64_at(d, 74).unwrap().max(cap);
            put64(d, 74, limit);
            put64(d, 90, cap);
            d[98..106].copy_from_slice(&s.clock.unix_timestamp.to_le_bytes());
            put64(d, 106, 0);
            let mut candidate = decoded(&cache, MARKETS[1].parse().unwrap()).await;
            candidate.load_reference_oracle(&cache).await.unwrap();
            let quote = candidate.quote(r.clone());
            let execution = candidate.reference_quote(&r);
            if accepted {
                let q = quote.unwrap();
                assert_eq!(
                    (q.expected_output, q.price),
                    (original_quote.expected_output, original_quote.price)
                );
                assert!(execution.unwrap().output >= q.expected_output);
            } else {
                assert!(
                    quote
                        .unwrap_err()
                        .to_string()
                        .contains("insufficient capacity for better actual execution")
                );
                assert!(execution.unwrap_err().to_string().contains("Custom(6027)"));
                assert!(
                    candidate
                        .generate_swap_instruction(r.clone(), Pubkey::new_from_array([211; 32]))
                        .is_err()
                );
            }
            // No new crossing gate or partial-range restriction, even with the
            // tight full-exit throttle: every sampled routed quote is identical.
            for (input, output) in [(0, 1), (1, 0)] {
                let (lo, hi) = v.bounds(input, output).unwrap();
                assert_eq!(candidate.bounds(input, output).unwrap(), (lo, hi));
                for i in 0..=128 {
                    let amount = lo + (hi - lo) * i / 128;
                    let req = QuoteRequest {
                        input_mint: v.get_token(input as usize).unwrap().pubkey,
                        output_mint: v.get_token(output as usize).unwrap().pubkey,
                        amount,
                        swap_type: SwapType::ExactIn,
                    };
                    let a = v.quote(req.clone()).unwrap();
                    let b = candidate.quote(req).unwrap();
                    assert_eq!((a.expected_output, a.price), (b.expected_output, b.price));
                }
            }
        }
    }
}
