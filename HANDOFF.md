# Ocelot integration handoff, 14 November 2026

## November 17 proportional redemption correction

Source `6152c3027ee03f81cb664fe2da750faf2dbec186` mirrors the new proportional V4 program planner. This is an
**un-deployed program candidate**; coordinate program, SDK and Titan rollout before
activation. The candidate executable is separately pinned as `506ef261…`; the original
deployed executable and all captured account files are retained.

The one-observation SOL3L reproduction improves from 46/61 rejected exits to 0/61.
All 732 candidate oracle-variation redemptions execute without overquotes. Whole-supply
coverage executes 1,230/1,230; captured and constructed worst-mid sweeps execute 1,470
and 52,262 comparisons with zero failures/overquotes/holes. Source verification includes
77 reference host tests, 44 default host tests, one creation-parser test, three explicit
routed/rollback tests, 40 shared Titan shape/construction checks and the two sweeps.

Every accepted output, marginal price and range in the 64,709-record regression remains
unchanged. Curves and worst-mid source are unchanged. Local release quote p99 batch means
are 142–301 ns; refresh p50 is 1.28–1.97 ms. See [identity, commands, exact coverage and
remaining constraints](PROPORTIONAL_REDEMPTIONS.md). The separate signed IOC price-limit
headroom issue and two constructed mint minimum-equity endpoint rejections remain.

Standalone export verification: 77 reference host tests and 45 default/creation tests
pass. Adapter/library Clippy with reference simulation and `-D warnings` passes;
formatting and whitespace checks pass. All 105 byte-identical source/export hashes match.
The deliberately broader `--all-targets` Clippy invocation hits two inherited
`result_large_err` warnings in unchanged `tests/common/mod.rs` through the upstream
example target; no shared test or error-type change was made to suppress them.
Explicit routed/rollback and crossing-sweep results above were run in the source checkout
against the identical exported runtime/test files and pinned candidate binary.

## Earlier handoff evidence

## November 17 current-state quoting follow-up

Source `c9c62091a2c787aa76372a89dd4a3a3d76d8c548` removes the speculative post-order mark/EMA model and recursive
redemption prefix. V4 cash-first redemption, the mint contribution cap,
conservative output curves, worst-mid pricing and captured-state risk checks stay.
Fresh SOL redemption ranges now reach essentially the full supply, instead of
the former 0.954197 SOL2S / 318.027904 SOL3L prefixes.

86 reference / 51 default / 40 unchanged shared shape/construction checks pass in
the source checkout. Whole-supply coverage records 418/418 successful fresh-state
redemptions and 806/812 historical-state executions, with six explicit risk
rejections. No successful execution returned less than quoted. Wider crossing
stress deliberately produces more execution-time risk rejections; they are
counted separately. Read [the exact metrics and contract](CURRENT_STATE_QUOTING.md).

Titan must simulate candidate routes and discard/rebuild failures. Its production
V4 allowlist, final minimum and Clock/coherent-cache delivery need acceptance.
The separate signed IOC limit headroom issue remains unresolved. The old fixed-EMA
premise was disproved; the earlier mark-bound and restricted-range evidence below
is historical. This is a review candidate, not routing activation or deployment.

Standalone export: 42 reference tests, 25 default-feature tests, both Clippy configurations and formatting pass. All 102 byte-identical source/export hashes match. Router/guard sources and fixtures are unchanged; this host-only follow-up does not require another SBF build.

## Earlier handoff evidence


## November 17 program compatibility

Source `3add741ce14f68a1b4f35546b796b05cff052270` updates the host adapter and template router for the deployed
Ocelot release `6acccee2…`. Redemption is V4 (V3 is retired); mint remains V3 with
the contributed-collateral issuance cap. Cash-first sizing/payout, configured IOC
limits, post-trade finance checks and conservative output/range preparation are
updated. Worst-mid remains with no extra five-bp haircut.

85 reference / 51 default / 40 shared shape and construction / 3 program tests
pass in the source checkout. Stress sweeps have zero overquotes or failed accepted
trades in 12,502 sampled executions plus 60 full exits, through 32-tick crossings.
Read [current derivation, evidence and limitations](PROGRAM_UPGRADE_2026_11_17.md),
including the conservative redemption-prefix availability cost, dust estimates
and the supported post-trade mark premise.

Titan needs to adopt the V4 discriminator in its production router and re-review
this candidate's issuer accounting, risk bounds and advertised ranges. The new
post-trade mark hull uses an oracle-anchored weighted-median premise that should
also be confirmed with Phoenix; the public SDK exposes layout, not its evaluator.
Host/cache/Clock delivery and production route-minimum acceptance remain open.
This is an engineering handoff, not routing activation or an Ocelot deployment.

Standalone verification completed on Rust 1.96.0: **43 reference tests,
29 default tests and 3 program/parity tests pass**. These include current-program
mint/redeem execution, V3 retirement, rewritten V4 close quantities, 240 altered
oracle executions, the normal quote/instruction golden, 165 routed executions,
and the test-only final-output rollback model. No RPC-dependent acceptance tests
are counted as successful after skipping. The separately ignored benchmark is
reported in the source validation record.

All **100 byte-identical export/source SHA-256 checks pass**, including Rust,
fixtures, manifests/locks and the new validation report. Documentation-only links
and handoff metadata are normalized. The independent router SBF rebuild matches
`6af1147c0a52e823daeae02683952ebf7400a9a3397b577f1c0a8f82f5374d31` exactly. The test-only output-guard ELF is unchanged. Public export
screening and whitespace checks pass. No private application or business notes
are exported.

## Earlier handoff evidence


## Worst-mid pricing in the adapter, 16 November 2026

Source `f14135fc3bac75bd1aaeac3e744489b83e7e7816` promotes the worst-mid prototype into the production adapter.
For raw locked/crossed books, buys use the highest live maker midpoint and sells
the lowest; the extra 5 bp output/price discount is removed entirely. Normal-book
pricing, the ordinary lot-boundary curve and original-price full-exit reserves
remain. Cache/strict-decoding fixes are retained.

82 reference / 51 default tests and 40 shared shape/construction checks pass.
Captured and controlled sweeps cover 2,893,312 quote/shape inputs and 16,440 sampled
execution comparisons plus 60 full exits with zero overquotes or execution
failures, through 32-tick controlled crossings. Worst sampled underquote is
14.5070 bp on captures / 111.5274 bp on constructed stress states. See
[current policy, sampling and timing](WORST_MID_VALIDATION.md).

The old common-midpoint/five-bp policy is superseded, not another active mode.
No on-chain source, instruction encoding, deployment or routing activation is
included. Titan's sampler cache confirmation and host acceptance remain open.

Standalone focused checks pass 34 reference / 22 default tests, both selected
Clippy configurations and formatting. Shared test binaries compile. All 95
byte-identical source/export hashes match and public-file screening passes.
The public runtime contains `live_mid_bounds()` selection and no
`OUTPUT_DISCOUNT_BPS` constant or quote discount branch.

Local worst-mid quote batch-mean p50 is 0.113–0.123 µs; refresh p50 is 1.741–1.819
ms. See the validation document for sampling and tail metrics. The normal-book
64,716-record quote/instruction baseline is unchanged. On-chain source/artifacts
were not changed or rebuilt. The earlier live Raydium compatibility run remains
historical evidence; current worst-mid validation uses Phoenix replay fixtures.


## Prior handoff evidence


## Strict RPC decoding, 16 November 2026

Source `bf39c48a30e2e0db5213d79c9b4ce109b5aaa74b` rejects unreadable non-null RPC accounts before publishing a
snapshot. Only actual RPC nulls represent absent accounts. This prevents corrupt
optional withdrawal-limit data from taking the unlimited-market-bucket path.
Tests cover valid compressed bytes, corruption, absence, atomic publication and
Ocelot refresh failure/recovery. Two regressions fail on the preceding code.

Pricing, instructions, on-chain programs and the common-midpoint/five-bp policy
are unchanged. `tests/common/mod.rs` still uses the prior pinned simulation
preparation; acceptance assertions are unchanged and the venue header now says
so explicitly. The separate route-test harness now pins its own quote/simulation
accounts too, after live validation exposed per-sample network reads. Its
assertions are unchanged. See [scope and evidence](CACHE_VALIDATION.md#strict-account-decoding).

Source and standalone focused tests pass 22 reference / 12 default checks,
including the 64,716-record frozen quote/instruction baseline. Both selected
Clippy configurations, formatting and shared test compilation pass. All 94
byte-identical export hashes match source; public-file screening passes.

The source checkout's full live `make test-example` passes without skips against
the public Solana mainnet RPC: eight reference tests, two creation/parser tests
and 20 exact-output comparisons in the routed test. The first live run hit
provider error -32016 during a per-sample read in the old route harness; its
replacement uses one pinned snapshot for all comparisons. No retry or relaxed
slot floor was added. Initial capture can still encounter provider lag.

The existing router SBF matches the previously recorded SHA-256
`b8997ff5ef34d8fa27a1e71fdfa1b134a6938788de920fa0e1d19b1f7d8652ba`.
No on-chain source changed, and the artifact was reused, not rebuilt/deployed.
These are local VM simulations with live-read state; no transactions were sent
to mainnet. The exact cache Titan's sampler used remains unconfirmed. No new
Ocelot live refusal rate, production-host fix or activation is claimed.


## Prior handoff evidence


## Cache review follow-up, 16 November 2026

Source `fbdc96b0a8b84cfe0800719a339a20a2ca459b32` fixes the shared simulation harness to pin quote preparation,
execution accounts and clock to one immutable RPC response. Fresh RPC batches
retain context slots, reject backwards responses and publish complete snapshots
atomically. Ocelot discovers instruction accounts independently of trade sizing
and clears prepared state before propagating required-key failures.

78 selected reference / 48 default tests and 40 shared shape/construction checks
pass, as do both selected Clippy configurations and formatting. Integrated
advancing-RPC and controlled orphan-index/buffer regressions pass. The frozen
64,716-record quote/instruction baseline is unchanged. See
[scope and evidence](CACHE_VALIDATION.md#review-follow-up-pinned-simulations-and-complete-discovery).

Shared Raydium binaries compile, but live replay is unrun without an RPC URL and
program dumps. The offline integration test invokes the actual generic snapshot
helper and replays Ocelot mint/redeem. Titan must confirm which cache its sampler
used and rerun; no new live refusal rate or production resolution is claimed.
Normal curves, common-midpoint/five-bp policy and instructions are unchanged.
Worst-mid remains separate. No on-chain changes or activation.

Standalone focused checks pass 19 reference / 10 default tests (two/one
benchmark tests respectively remain ignored), both selected Clippy configurations
and formatting. The shared Raydium/Ocelot binaries compile. All 94 byte-identical
export hashes match the committed source; only documentation links/references and
export metadata differ. Public-file screening passes. No live Raydium or new
latency/availability measurement is claimed.

## Prior handoff evidence


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
