# Worst-mid pricing, 16 November 2026

This promotes the previously isolated worst-mid prototype into the tracked
production adapter. It replaces common-midpoint recentering and removes the
additional five-bp output/price multiplier. It does not change on-chain source,
transaction encoding, router slippage handling or activate routing.

## Implementation

- Raw best bid below best ask, or a one-sided book: retain the normal native path.
- Raw best bid equal to or above best ask: recenter makers to the highest live
  maker midpoint for buys and the lowest for sells. Mint/redeem select the side
  of the actual hedge order, including short products.
- The existing validated spline parser collects midpoint extrema while parsing
  active nonzero-mid makers with an unfilled/unexpired region on either side.
  Selection precedes maker risk caps, matching the prototype. Fully consumed,
  expired, inactive and zero-density regions do not contribute. The existing
  saturating-expiry/include-future policy is preserved; exact expiry is live.
  A missing bound returns a preparation error rather than a prototype assertion.
- Original accounts remain unchanged. Model preparation preserves spline widths,
  region offsets, fills and risk fields, and retains native maker/taker checks,
  FIFO reserves, fees, collateral/withdrawal limits and original-price full-exit
  reserves. No crossing-width cutoff or blanket availability restriction is added.
- The ordinary conservative lot-boundary curve and its marginal price are returned
  directly. The extra discount constant and application branch were deleted.
  The legacy `uses_midpoint_approximation` diagnostic now identifies worst-mid;
  it does not apply a discount or claim a mathematical output guarantee.

## Validation

- 82 selected reference-feature tests, 51 default-feature tests and 40 unchanged
  shared curve/construction checks pass. Both selected Clippy configurations and
  formatting pass. Benchmarks and the explicit dense/sparse sweeps are separate.
- The frozen normal-book baseline matches all 64,716 records, including 487
  instructions. Full-exit insufficient-capacity controls and partial-range
  invariance remain covered with the new pricing model. No guards were relaxed.
- Captured locked/crossed books: 188,680 quote/shape inputs, 1,096 sampled execution
  comparisons and four full exits, zero overquotes/failures/range holes. Largest
  sampled underquote: 14.5070 bp.
- Controlled widths 0/1/2/4/8/16/32 ticks, both crossing directions, four fixtures:
  2,704,632 quote/shape inputs, 15,344 sampled execution comparisons and 56 full
  exits, zero overquotes/failures/range holes. Largest sampled underquote:
  111.5274 bp in constructed stress states. These results reproduce the isolated
  prototype's per-case aggregates on the current cache-fix integration.
- Sweeps used `MIDPOINT_EXECUTION_STRIDE=512`. Every generated lot-edge/interior
  shape check still runs; execution sampling also includes endpoints, fixed and
  deterministic random sizes. Full-market exits are checked separately.
- Unit coverage exercises inactive, zero-mid, fully filled, expired, zero-density,
  one-sided, exact-expiry and future-slot makers, plus zero risk caps. No prototype
  assertions or duplicate unchecked spline parsing remain in preparation.

Release timing on this machine (100 samples, 10,000 alternating quotes per batch):

| State | Quote batch-mean p50 / p99 | Refresh p50 / p99 |
| --- | --- | --- |
| Normal SOL3L | 0.115 / 0.280 µs | 1.309 / 2.294 ms |
| Locked SOL3L | 0.123 / 0.131 µs | 1.819 / 2.014 ms |
| Crossed SOL3L | 0.123 / 0.133 µs | 1.813 / 2.053 ms |
| Crossed SOL2S | 0.113 / 0.124 µs | 1.741 / 1.864 ms |

Quote percentiles describe batch means, not individual-call tail latency. These
are local timings, not Titan-host measurements. Quoting remains allocation-free.

The old common-midpoint counterexamples and full-exit cases now require output
at or below Phoenix execution, including an instruction with the quoted minimum.
Captured and controlled sweeps both enforce zero overquotes, execution failures
and range holes. Their chord/MVT/monotonicity assertions remain active at every
sampled lot boundary and midpoint; only expensive VM execution sampling is
explicitly configurable with `MIDPOINT_EXECUTION_STRIDE` (defaults unchanged).

## Scope and remaining acceptance

These are captured-state and constructed-stress comparisons against the reference
VM with deployed program fixtures, not a universal proof of all slide-price or
risk/capacity states. Wider crossings can produce substantially lower quotes.
Titan must review the output-error/availability policy and production cache/router
integration. Sampler-cache confirmation remains outstanding. ANT requires separate
validation; no new live availability claim or activation is made here.

The previous [policy and evidence](MIDPOINT_VALIDATION.md) are retained as history.
Cache decoding and snapshot fixes are retained unchanged; their prior live
Raydium compatibility check is recorded in [cache validation](CACHE_VALIDATION.md).
