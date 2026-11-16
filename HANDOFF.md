# Ocelot integration handoff, 14 November 2026

## Cache-coherence follow-up, 16 November 2026

Source `20a9849862e14cabb8d2b9424f289c2a8e734bc8` batches discovery dependencies with the clock and withdrawal
queue, and refreshes every RPC batch key instead of reusing earlier cache hits.
The production host supplies its own coherent AccountsCache; the adapter adds no
RPC calls and does not require identical last-write slots. Cold discovery uses
three batch calls; normal warm refreshes use two. The RPC helper fetches one full
response per batch (maximum 100 accounts) and never falls back to stale hits.

74 selected reference / 46 default tests and both Clippy configurations pass.
Three new regressions fail before this fix; the frozen 64,716-record quote and
instruction baseline is unchanged. See [scope and evidence](CACHE_VALIDATION.md).
The normal curves, common-midpoint/five-bp policy, guards and instructions remain
unchanged. Worst-mid pricing is a separate proposed update. No activation.

Standalone focused tests pass 15 reference and 8 default cases, with
both Clippy modes and formatting passing. All 93 unmodified-export files
match source SHA-256 hashes; documentation link normalization and public screening
pass. No live refusal-rate or new performance measurement is claimed.

## Prior handoff evidence


This update enables native midpoint quotes for locked/crossed Phoenix spline
books, using the selected five-bp final-output discount. Normal-book output,
price, ranges and instructions are unchanged. No Hawkeye, local VM or RPC runs
in preparation or quoting; research-only experiments are excluded from this
standalone package. This is submitted for engineering review, not activation.

## Full-exit review follow-up

The complete-close withdrawal reserve now uses the more favorable of original
raw-book and model prices. This fixes the reproduced SOL2S throttle false accepts
without changing quote curves, the five-bp adjustment, ordinary routing ranges,
instruction generation or the accepted crossing policy. Only exact full-market
redemptions can newly decline when capacity is below the favorable-price reserve;
this includes some conservative declines if actual execution would fit.

Both regressions fail on `0581d19` and pass now. All 12 recorded unthrottled
full-exit quote outputs are unchanged, and sufficient-capacity controls execute.
Another 1,032 partial quote/price comparisons and a 64,716-record normal baseline
match exactly. Clock-only expiry must refresh successfully, with a dedicated
midpoint-to-normal pricing transition test. Source checks pass 71 reference,
44 default and 40 shared tests plus both Clippy configurations and formatting.

Earlier dense sweeps covered advertised routing ranges, excluding full-market
exits. Those are now separate records in both sweep modes. Added full-exit-only
runs execute 4 captured and 56 controlled exits without failures; 12 controlled
quotes remain above execution (worst 43.7741 bp), within the disclosed approximation
policy rather than a new price correction. See [review evidence](MIDPOINT_VALIDATION.md#review-follow-up-full-market-exit-capacity)
and [full-exit measurements](FULL_EXIT_MEASUREMENTS.json).

## Implementation

- `update_state` reads one supplied account batch. Normal geometry retains the
  prior conservative provider. Locked/crossed geometry prepares common-midpoint
  models, rounding up for buys and down for sells, retaining maker widths,
  offsets, expiry and risk fields. Integer midpoints share one prepared model.
- The venue keeps original accounts. Existing native risk/capacity, finance,
  full-fill and range checks remain active against the model. Invalid/missing
  dependencies or stale oracle data still fail closed and clear prior quotes.
- Quote evaluates the prepared curve, then returns
  `floor(output * 9995 / 10000)` and `price * 0.9995` only on the approximation
  path. It remains allocation-free. The discount changes the estimate, not
  fees, user balances or the actual amount delivered by Ocelot.
- The exact-input v3/CPI contract is unchanged. Routed legs use zero per-leg
  minimum and depend on Titan's final user minimum. Standalone builders require
  a positive caller minimum. The required deadline remains `i64::MAX` as agreed.
  `bounds()` remains necessary for contiguous finance-safe modeled ranges.

## Error policy requiring review

**Five bps does not guarantee that every crossed-state quote is below execution.**
Common-midpoint pricing is not Phoenix slide pricing, and modeled executable
capacity is not an independent proof of actual capacity. The default production
provider retains regressions demonstrating residual overquotes in controlled
four-tick and wider crossings. For example, an eight-tick fixture still overquotes
by 8.8924 bp after discount; the 32-tick stress sweep reaches 51.1436 bp.
These controlled mutations are not a measured live failure rate or proof of
reachability through every valid maker update sequence. No crossing-width gate
or adaptive buffer is included in the selected policy.

The unmodified captured locked/crossed SOL books pass 188,680 actual execution
comparisons with no overquotes or failures; maximum underquote is 15.8219 bp.
The controlled sweep checks 2,704,632 quote inputs and compares 52,136 executions:
11,444 wider-crossing overquotes remain, with no permissive-floor execution
failures. These are sample results, not universal safety guarantees. Titan's
final user minimum must reject insufficient actual output atomically.

See [policy, examples and reproduction](MIDPOINT_VALIDATION.md),
[machine-readable measurements](MIDPOINT_MEASUREMENTS.json),
[normal curve derivation](CURVE_VALIDATION.md),
[standalone export verification](EXPORT_VALIDATION.md) and
[source provenance](SOURCE.json).

## Requested Titan review

1. Review/accept the explicit five-bp approximation and residual-error tradeoff
   against your private harness, including locked/crossed geometry and both
   SOL3L/SOL2S mint/redeem directions. Earlier native-provider review does not
   establish correctness of this new path.
2. Verify host integration invokes the subscribed refresh on Clock-only changes
   and uses `venue.bounds()`; these behaviors remain necessary even with the
   confirmed coherent account batch.
3. Verify the production router's final user minimum/atomic rollback, including
   0.5% user slippage and a result below that floor after Ocelot succeeds.
   Our test-only guard is not Titan's production guard.
4. Confirm quote/error and refresh-budget acceptance on production hardware.
   Prior local quote medians are about 0.11–0.12 µs; full cached refresh medians are
   1.34–1.89 ms, p99 up to 3.67 ms. Quote p99 measures batches, not individual calls.

The source suite passes 71 reference, 44 default and 40 unchanged shared checks.
The prior three program/parity checks and SBF identities remain applicable;
program sources are unchanged and were not rebuilt for this follow-up. The route guard exercises 144 successes and 96 atomic
rollbacks across normal/locked/crossed fixtures. Timestamp-only tests give 32
+21-second successes and 16 expired controls; fixed slots do not establish
liquidity survival after wallet signing. Shared old-VM execution tests are not
claimed as passed. Runtime/ABI compatibility, delay handling and ANT's unsupported
oracle profile remain outside this update's validation. Neither PR is merged and
no routing activation or on-chain upgrade is included.
