# Full-exit capacity follow-up export, 14 November 2026

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


Source `2afd0107e1fbfbe1fd904c67ef1e44a223feb8b6` preserves favorable original-book prices for the complete-close
withdrawal reserve. Quote curves, the five-bp discount, partial routing ranges,
instructions and the approximation/availability policy are unchanged. The new
admission check is restricted to full-market exits with insufficient reserved
capacity; some conservative declines are possible near that bound.

Source verification: 71 reference / 44 default / 40 shared tests pass, both Clippy
configurations and formatting pass. Both new capacity regressions fail at
`0581d19`. Full-exit-only runs add 60 successful original-account executions,
including 12 retained controlled-state overquotes. The old dense range sweeps
and benchmarks are historical, not rerun for this capacity-only change.

Standalone verification passes: **9 focused reference tests, 6 focused default
tests**, both Clippy configurations with warnings denied, and formatting. Rust,
fixtures, tests, manifests/locks and measurements are byte-identical to the
committed source; shared assertions match pinned upstream and all exported files
pass screening. Dependency/build caches were reused, with the exported package
recompiled from this checkout. The complete 71/44/40 source suites were not
repeated after a byte-identical export; this is not a clean-machine install.

Focused reproduction:

```sh
cargo +1.96.0 test --profile release-debug --locked --features reference-simulation --test ocelot_full_exit --test ocelot_midpoint --test ocelot_quote_regression
cargo +1.96.0 test --profile release-debug --locked --test ocelot_full_exit --test ocelot_midpoint --test ocelot_quote_regression
```
Program sources, shared assertion source and SBF artifacts are unchanged; no SBF
rebuild or activation is claimed. The detailed changes and limitations are in
[MIDPOINT_VALIDATION.md](MIDPOINT_VALIDATION.md) and
[FULL_EXIT_MEASUREMENTS.json](FULL_EXIT_MEASUREMENTS.json).

## Historical standalone midpoint export validation, 14 November 2026

Exported source: `25cdb9d81afa0e960bbfcd6b1471838836ab4ead`. The default adapter now uses direction-aware native
midpoint preparation with a five-bp final-output discount only for locked/crossed
books. Normal quotes and the on-chain contract are unchanged. This is an explicit
approximation with known overquotes, not a universal execution-output bound.

## Current standalone results

| Check | Result |
| --- | --- |
| Reference-feature `release-debug` suite | 67 passed, allocation guards enabled |
| Default-feature `release-debug` suite | 42 passed; 4 benchmarks explicitly ignored |
| Unchanged shared assertions, eight fixture/market cases | 40 passed |
| Program host/enum parity tests | 3 passed |
| Reference/default Clippy with warnings denied | Both pass |
| Formatting | Pass |
| Rust/tests/manifests/locks/measurements/fixtures | Byte-identical to committed source |
| Shared quote/router assertion files | Byte-identical to pinned Titan upstream |
| Router/test-guard artifacts | Identical to prior independently rebuilt artifacts |
| Standalone source/privacy screen | Pass |

The reference selection excludes named benchmark/live research cases and the
expensive dense execution sweep. Its five midpoint tests exercise the default
provider, unchanged normal output/instructions, state preservation, refresh mode
transitions, expiry, original-state executions and retained wider-crossing error
regressions. It includes 144 routed successes and 96 atomic rollbacks via a
**test-only** final guard, plus 32 timestamp-only +21-second successes and 16
expired controls. This does not test Titan's actual production guard.

The 40 shared checks cover construction/allocation, zero-input price, monotone
output/price and mean-value theorem. The old-VM execution and shared speed cases
are explicitly excluded; compatible actual-program replays and the native
benchmark supply separate evidence. Nothing skipped is counted as passed.

The dense 188,680 captured executions and controlled 52,136-execution / 2,704,632
quote study were run in the source workspace, not rerun after a byte-identical
export. Original captured books have no overquotes/failures; the controlled
study retains 11,444 overquotes in wider crossings. These are not a production
failure probability or a universal five-bp safety bound. Details and commands:
[MIDPOINT_VALIDATION.md](MIDPOINT_VALIDATION.md),
[MIDPOINT_MEASUREMENTS.json](MIDPOINT_MEASUREMENTS.json).

Cargo dependency/build caches were reused; the exported package was recompiled
from this checkout, as recorded in build logs. This is not a clean-machine
installation. Program source is unchanged, so no new SBF rebuild is claimed.
The existing separately rebuilt router/guard match the source-workspace files:

- Router: `b8997ff5ef34d8fa27a1e71fdfa1b134a6938788de920fa0e1d19b1f7d8652ba`
- Test-only guard: `9f5383d9036a493f3132dfdb4090ebd479f5237f7c5d198d6fd9215436bb090c`

## Current standalone performance

100 batches of 10,000 complete quotes and 100 complete cached refreshes per case.
Quote percentiles describe batch means, not individual-call tails. Local Mac
measurements are not production latency guarantees; normal-book timing includes
an outlier batch above 1 µs while all p99 batch means remain below 1 µs.

| Snapshot | Quote median / p99 / max (µs) | Refresh median / p99 (ms) |
| --- | ---: | ---: |
| Normal SOL3L | 0.110 / 0.415 / 1.264 | 1.302 / 2.095 |
| Locked SOL3L | 0.120 / 0.131 / 0.133 | 1.388 / 1.521 |
| Crossed SOL3L | 0.120 / 0.127 / 0.129 | 1.827 / 2.031 |
| Crossed SOL2S | 0.122 / 0.143 / 0.145 | 1.779 / 2.035 |

## Commands run

Run from the standalone package root; the shared tests were served by
`tests/support/fixture_rpc.py` using the eight cases documented in
[MIDPOINT_VALIDATION.md](MIDPOINT_VALIDATION.md).

```sh
cargo +1.96.0 test --profile release-debug --locked --features reference-simulation --lib --test ocelot --test ocelot_native --test ocelot_phoenix --test ocelot_state --test ocelot_replay --test ocelot_spline --test ocelot_slippage --test ocelot_curve --test ocelot_midpoint --test your_venue_creation -- --include-ignored --skip benchmark_ --skip fresh_spline_comparison --skip production_execution_sweep
cargo +1.96.0 test --profile release-debug --locked --lib --test ocelot --test ocelot_native --test ocelot_phoenix --test ocelot_state --test ocelot_spline --test ocelot_slippage --test ocelot_curve --test ocelot_midpoint --test your_venue_creation
cargo +1.96.0 clippy --locked --features reference-simulation --lib --test ocelot_midpoint --test ocelot_curve --test ocelot_slippage --example capture_locked -- -D warnings
cargo +1.96.0 clippy --locked --lib --test ocelot_midpoint -- -D warnings
cargo +1.96.0 fmt --check
cargo +1.96.0 test --locked --manifest-path program-template/Cargo.toml -p titan-v3-venue-template --lib --test venue_parity
cargo +1.96.0 test --profile release-debug --locked --test your_venue --no-run --message-format=json
cargo +1.96.0 test --release --locked --test ocelot_midpoint benchmark_midpoint_quote_and_refresh -- --ignored --nocapture
# Run the built shared test binary against each read-only fixture RPC:
SOLANA_RPC_URL=http://127.0.0.1:PORT OCELOT_TEST_MARKET=MARKET your_venue_test_binary --skip bound_simulation --skip random_samples --skip quoting_speed --nocapture
```

Titan review must accept the approximation/error policy and verify production
Clock delivery, range discovery, final user minimum and runtime/ABI integration
before activation. No routing activation, program upgrade or ANT validation is
implied by this export. The prior evidence below is retained as history.

## Historical standalone curve export validation, 12 November 2026

The checks below passed from the standalone checkout exported from source
`b597dddf7c5ea8bd053e7cf8507dff8ee478e84e`. Runtime and test sources, Cargo manifests/locks and all four
public-chain captures match that source byte-for-byte; documentation links and
submission status are adapted for standalone use. No application history,
business notes or credentials are included.

## Current verification

| Check | Result |
| --- | --- |
| Selected reference-feature suites | 62 passed; benchmarks/fresh-RPC diagnostics excluded |
| Selected default-feature suites | 38 passed; 3 benchmarks/diagnostics ignored |
| Program host and enum parity | 3 passed |
| Unchanged shared assertions via fixture RPC | 20 passed across four captures; no live-RPC claim |
| Curve lot-edge/interior comparisons | 273,390 accepted; no monotonicity, price or MVT violations |
| Synthetic accounting comparisons | 9,666 accepted |
| Stressed finance-range comparisons | 2,040,435 accepted; 24/80 stressed directions conservatively unavailable |
| Expanded captured-execution sweep | 578 accepted, zero overquotes or failed executions; maximum 11.699366 bp underquote |
| Stressed range execution | 616 accepted, zero overquotes or failed executions |
| Original captured execution sweep | 205 accepted and 39 matching declines; maximum underquote now 11.804777 bp |
| Existing routed parity executions | 51 pass |
| Final user-minimum model | 72 successes, 48 complete rollbacks |
| Deadline isolation | 16 successes at +21 seconds, 8 expired-deadline controls |
| Clippy, default/reference selections | Both pass with warnings denied |
| Formatting / whitespace / export source parity | Pass |
| Shared quote/router assertion sources | Byte-identical to pinned upstream |
| Router and test-guard artifacts | Match prior independently rebuilt artifacts; program sources unchanged |

Core curve math received internal review. The later mint-range and per-direction
failure-isolation fixes are included for Titan engineering re-review. The accepted
execution sweeps are same-state evidence, not guarantees after wallet delay. The
reviewer's additional mutated states reached 23.7 bp underquote; neither that nor
12 bp is a universal bound. Locked/crossed books still fail closed.

The 20 shared checks cover construction/allocation, zero-input spot price,
monotone output, monotone price and MVT, served by the read-only fixture RPC.
Shared bound/random execution simulations were filtered because the old VM
cannot load current Phoenix executables; the shared speed test was filtered in
favor of the complete native benchmark below. No skipped test is counted as a
pass. The compatible execution reference and test-only final guard do not
validate Titan's production router or private harness.

Cargo dependency/build caches were reused, with the exported package recompiled
from this checkout; the build log records its standalone source path. This is
not a clean-machine dependency installation. No program source changed, so no
new SBF rebuild was needed; the existing independently rebuilt router and guard
were verified against source-workspace artifacts:

- Router: `b8997ff5ef34d8fa27a1e71fdfa1b134a6938788de920fa0e1d19b1f7d8652ba`
- Test-only guard: `9f5383d9036a493f3132dfdb4090ebd479f5237f7c5d198d6fd9215436bb090c`

## Current standalone benchmark

```text
complete OcelotVenue::quote: 1,000,000 calls, batch-mean p50=115.7ns p99=120.8ns max=121.3ns (nearest-rank; refresh excluded)
native preparation: 100 calls p50=331.3us p99=407.8us max=462.1us (nearest-rank; cached-state decoding, excludes cache fetch/copy)
complete cached update_state: 100 calls p50=1276.2us p99=1531.3us max=1546.1us (nearest-rank; in-memory Snapshot cache, no network)
cached refresh fan-out: 4 fixture instances, 100 cycles p50=5.016ms p99=5.438ms max=6.165ms (nearest-rank; sequential; excludes network)
cached refresh fan-out: 26 fixture instances, 100 cycles p50=32.651ms p99=38.599ms max=54.117ms (nearest-rank; sequential; excludes network)
individual complete quotes: 100,000 calls p50=125ns p99=167ns p99.9=250ns max=14583ns; timer-only p50=0ns p99=42ns (nearest-rank; includes timer overhead and scheduling)
```

These measurements use a release build, two alternating supported quote inputs
and an in-memory account cache without network delivery. Batch means are not
individual-call worst-case latency. Individual timings include timer overhead
and scheduling. Fan-out repeats four fixture states rather than 26 distinct
live markets. Range discovery and preparation are outside quote timing; no
production latency guarantee is implied.

## Reproduce and review

Use [CURVE_VALIDATION.md](CURVE_VALIDATION.md#reproduce) for current suite and
fixture-RPC commands, [README.md](README.md) for router/guard builds, and
[HANDOFF.md](HANDOFF.md) for the re-review scope and acceptance questions. Run
`shasum -a 256 -c SHA256SUMS` to verify the export manifest. The historical
checkpoints below retain their original results and are superseded by this
section for the current curve candidate.

## Historical export validation, 10 November 2026

The selected checks below passed from this standalone checkout. The Rust source,
Cargo manifests/lockfiles and all four fixture archives match the source commit
in SOURCE.json byte-for-byte. The latest adapter source is
`52e54dfc0b4ed883722fee62f5fa234586194a53`. The prior quote/execution review cleared
`a8c5f9fd`; the later adapter-policy patches is not covered by that sign-off.
Documentation links/status are normalized for the public export, and HANDOFF.md
contains the screened technical handoff. No application source/history, business
notes or credentials are exported.

### Checks

| Check | Result |
| --- | --- |
| Selected reference-enabled host suite | 49 passed |
| Selected default-feature suite | 27 passed; 3 benchmarks ignored |
| Program host and enum parity | 3 passed |
| Native quote vs captured execution | 205 accepted executions; 39 matching declines; no overquotes or false accepts |
| Routed execution | 51 existing parity executions across 17 cases and 3 reserve modes |
| Final minimum/rollback model | 72 successes and 48 complete rollbacks after Ocelot/Phoenix execution |
| Standalone minimum | Positive caller floor required; zero rejected by standalone helper |
| Prior-source regressions | Zero-floor and maximum-deadline assertions fail on their preceding sources |
| Deadline isolation | 16 direct/routed successes at +21 seconds; 8 expired-deadline rejections |
| Zero-weight regression | 80 accepted executions; 48 matching rejections; 16 conservative declines |
| Clippy, default and reference-enabled selections | Both passed with warnings denied |
| Rust formatting / diff whitespace | Passed |
| SBF router and test-guard identity | Prior independent builds match; program sources and hashes unchanged by deadline patch |
| Exported source and fixture parity | Verified |
| Upstream shared quote/router assertions | Byte-identical to the pinned template |

The test-only final guard is appended after the route in one transaction. It
checks the output delta against a harness-supplied initial balance. It is not
Titan production code and must never be deployed. Failed transactions restore
all writable route accounts, allowing only the payer fee deduction. See
[SLIPPAGE_VALIDATION.md](SLIPPAGE_VALIDATION.md) for coverage and the production
acceptance request. The actual Titan production guard has not been exercised.

The deadline regression changes only Unix time, holding slot-dependent state
fixed: all four captures and both directions execute directly and through the
router after 21 seconds. An expired deadline still rejects eight controls with
`DeadlineExpired`. The historical expiry regression explicitly retains its old
bounds. This isolates the cutoff; it does not guarantee liquidity survival or
blockhash validity over a real delay.

Host checks use Rust 1.96.0 with locked dependencies. Cargo dependency caches were
reused; the integration package's prior host artifacts were cleaned before
recompiling the exported sources. This is standalone-source verification, not a
clean-machine install test.
The SBF router was independently rebuilt for the preceding slippage handoff
with cargo-build-sbf 2.3.0, platform tools v1.54 and `--arch v0`. Its source and
artifact are unchanged by this host-only deadline patch. SHA-256:
`b8997ff5ef34d8fa27a1e71fdfa1b134a6938788de920fa0e1d19b1f7d8652ba`.

The unchanged, previously independently rebuilt test-only guard has SHA-256
`9f5383d9036a493f3132dfdb4090ebd479f5237f7c5d198d6fd9215436bb090c`.

The shared RPC-dependent acceptance suites were not run. Benchmarks and optional
live-capture tests were excluded from the selected correctness suite; the prior
quote benchmark below is retained because this patch does not change quote math. No funds, signatures,
deployment, live routing or acceptance-policy changes were involved.

### Earlier November 10 export benchmark (unchanged quote implementation)

Recorded before the slippage patch on the local development host; not rerun for this instruction-only policy change:

```text
complete OcelotVenue::quote: 1,000,000 calls, batch-mean p50=94.9ns p99=104.6ns max=105.5ns (nearest-rank; refresh excluded)
native preparation: 100 calls p50=354.6us p99=482.3us max=516.3us (nearest-rank; cached-state decoding, excludes cache fetch/copy)
complete cached update_state: 100 calls p50=1271.0us p99=1706.0us max=1765.5us (nearest-rank; in-memory Snapshot cache, no network)
cached refresh fan-out: 4 fixture instances, 100 cycles p50=5.201ms p99=6.458ms max=6.576ms (nearest-rank; sequential; excludes network)
cached refresh fan-out: 26 fixture instances, 100 cycles p50=33.794ms p99=57.105ms max=89.085ms (nearest-rank; sequential; excludes network)
individual complete quotes: 100,000 calls p50=125ns p99=167ns p99.9=250ns max=26417ns; timer-only p50=0ns p99=42ns (nearest-rank; includes timer overhead and scheduling)
```

Quotes use two alternating supported fixture inputs. Batch means exclude refresh;
individual-call timings include timer overhead and scheduling. The timer-only p99
is 42 ns. In-memory cache refresh excludes network delivery; fan-out repeats the
four captured market instances rather than measuring 26 different live markets.
These measurements are not production latency guarantees. Quote and refresh
latency are separate budgets for Titan to confirm.

### Reproduce

Run from the standalone package root. Build the router before routed replay.

```sh
(cd program-template && cargo-build-sbf --tools-version v1.54 --arch v0 --manifest-path programs/titan-v3-venue-template/Cargo.toml)
(cd program-template && cargo-build-sbf --tools-version v1.54 --arch v0 --manifest-path test-programs/output-guard/Cargo.toml)
cargo +1.96.0 test --locked --features reference-simulation --lib --test ocelot --test ocelot_native --test ocelot_phoenix --test ocelot_state --test ocelot_replay --test ocelot_spline --test ocelot_slippage --test your_venue_creation -- --include-ignored --skip benchmark_ --skip fresh_spline_comparison
cargo +1.96.0 test --locked --lib --test ocelot --test ocelot_native --test ocelot_phoenix --test ocelot_spline --test ocelot_slippage --test your_venue_creation
cargo +1.96.0 test --locked --manifest-path program-template/Cargo.toml --lib --test venue_parity
cargo +1.96.0 clippy --locked --lib --test ocelot --test ocelot_native --test ocelot_phoenix --test ocelot_spline --test ocelot_slippage -- -D warnings
cargo +1.96.0 clippy --locked --features reference-simulation --lib --test ocelot_state --test ocelot_phoenix --test ocelot_replay --test ocelot_native --test ocelot_spline --test ocelot_slippage -- -D warnings
cargo +1.96.0 fmt --check
shasum -a 256 -c SHA256SUMS
```

See HANDOFF.md for the remaining host/routing policy decisions and NATIVE_QUOTING.md
for conservative refusals, pricing error, delayed execution and ANT scope.
