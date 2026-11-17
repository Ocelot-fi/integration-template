# Current-state quoting, 17 November 2026

**Latest candidate: [proportional redemption correction](PROPORTIONAL_REDEMPTIONS.md).**
This adapter now mirrors an un-deployed program fix that closes the redeemed share of
exposure, rounded up, instead of spending leverage-band headroom. The conservative curve
and worst-mid adjustment remain unchanged. Coordinate program/SDK/Titan rollout before
activation. The earlier planner's execution-rejection counts below are historical;
updated reproduction, coverage and remaining limitations are in the linked record.

This follow-up removes the speculative post-order reference-price forecast from
the November 17 compatibility patch. It does not change the deployed Ocelot
program or Titan's template router. V4 redemption and V3 mint remain supported.

## Behavior

The adapter prices the trade from the supplied coherent account state. It keeps
the V4 cash-first minimum-lot planner and payout math, contributed-collateral mint
cap, conservative concave output curves and matching marginal prices, worst-mid
pricing without a five-bp haircut, oracle validity checks, maker/capacity checks,
withdrawal budgets, and finance checks evaluated at the supplied mark.

Removed: `live_mark_bounds`, hypothetical-mark mint/risk checks and constraints,
and recursive `redemption_prefix` interval certification. The prior fixed-EMA
premise is false: a pinned execution changes EMA 0 to -1 and mark 7454 to 7453.
The new regression preserves that counterexample. Output curve construction is
unchanged; removing forecast constraints changes which sizes can be quoted.

`bounds()` still applies captured-state mint finance constraints and normal
quote eligibility. It no longer cuts redemptions at the first hypothetical-mark
failure. It is not a proof that all sizes execute after Phoenix updates its mark.
A current-state quote may fail the program's execution-time risk checks, even
with the supplied accounts unchanged before execution. No post-order mark or EMA
rule is modeled, and no probability of such failures is assumed.

Titan must simulate candidate routes, discard/rebuild ones that fail, and enforce
the user's final minimum atomically. `generate_swap_instruction()` still encodes
zero per-leg minimum for that protected route only; standalone construction
requires a positive minimum. Simulation is not a guarantee against later changes.
Clock/cache coherence, refreshes and signing-delay recovery still matter.

## Redemption coverage

The unmodified slot-470602378 fixture now advertises:

| Market | Previous maximum | Current range (token units) | Total supply |
| --- | ---: | ---: | ---: |
| SOL3L | 318.027904 | 0.000704–4,221.144704 | 4,221.144761 |
| SOL2S | 0.954197 | 0.000192–9.991616 | 9.991624 |

The range search rounds its endpoints conservatively by a few atoms. Individually
quoted full exits remain supported. These are fixture-specific capacity figures,
not general liquidity promises. The former SOL2S maximum corresponded to only
about $1.46; that forecast-induced restriction is removed.

`ocelot_availability` samples 200 evenly spaced amounts over the WHOLE supply per
state, endpoints, full exits and the reported false-decline cases. It checks
execution even outside `bounds()`, rather than hiding those amounts with `continue`.

- Fresh capture: 418/418 sampled redemptions execute; no executable input is
  declined and no successful output is below its quote.
- Four historical states with the current executable substituted: 806/812 execute;
  six accepted quotes hit the program's retained-risk check. No executable input
  is declined and no successful output is below its quote.
- All six rejected `(fixture, amount)` pairs are pinned in the regression. The
  test fails on any new rejection, unexpected error site, quote hole or overquote.
- A separate altered-oracle control intentionally quotes a trade whose post-order
  risk check rejects. Another control reproduces the EMA counterexample and
  succeeds with conservative output.

These sampling rates are not live-market probabilities or uniform coverage of
every input atom. Historical states are not fresh captures. In particular,
execution failures are not silently counted as successful output comparisons.

## Validation

- 86 selected reference-enabled host tests, 51 default-feature tests and all 40
  unchanged Titan shared shape/construction assertions pass. The latter run
  across eight fixture/market combinations; the template's incompatible original
  VM tests and its timing test are excluded from that shared run. Actual execution
  is checked separately against the compatible, pinned reference runtime.
- 502,975 normal lot-edge/interior output comparisons and 2,521,355 stressed
  finance-range quote/chord checks pass. Five of 80 stressed directions remain
  unavailable under the captured-state constraints.
- Normal curve breakpoint/size execution: 575 successes with no overquote, plus
  one explicitly pinned risk rejection. Stressed finance endpoints: 823 successes
  with no overquote, plus two pinned minimum-equity margin rejections.
- Captured locked/crossed sweep: 188,790 quote/shape checks, 1,086 successful
  sampled executions, eight retained-risk rejections, and four successful full
  exits. Zero overquotes or holes in the advertised ranges.
- Constructed crossings through 32 ticks: 2,706,046 quote/shape checks, 13,628
  successful sampled executions, 1,674 execution-time risk rejections, and 56
  successful full exits. Zero overquotes or range holes. This intentionally
  adverse sample must not be used to estimate a live rejection probability.
- Execution sampling uses stride 512, endpoints, adjacent lot edges and seeded
  amounts. It is not exhaustive execution of every input. The sweep permits only
  the pinned program's retained-risk/margin rejection sites; IOC, slippage and
  unrelated execution failures still fail the test. Its raw rejection count is
  reported separately, without asserting that Titan has accepted that rate.
- Host Clippy with/without reference simulation, formatting and whitespace checks
  pass; only the existing upstream large-error Clippy lint is exempted.
- Updated quote/instruction baseline: 64,709 records, SHA-256
  `43c34f24af5754ed44e0eb7dd177648199e628da80ecbed44a7e4ce00a2fa93b`.
  The previous hash is preserved as historical evidence, not used as validation
  of changed behavior. Independent execution/coverage checks above provide that.

Local release timing after the parallel builds finished: quote batch-mean p50
122–137 ns / p99 139–151 ns; refresh p50 1.32–1.89 ms / p99 1.95–2.08 ms;
both bounds together p50 14.8–17.1 microseconds. The first timing run overlapped
other compilation/execution and failed its p99 gate; the separate rerun passed
all four fixtures. These local batch timings are not production tail guarantees.

The on-chain router and test guard sources are unchanged from the prior verified
export. No new SBF build or deployment is needed for this host-only change.

Reproduce the new coverage and curve checks:

```sh
cargo +1.98.1 test --locked --features reference-simulation \
  --test ocelot_availability --test ocelot_curve --test ocelot_upgrade \
  --test ocelot_quote_regression -- --nocapture
MIDPOINT_EXECUTION_STRIDE=512 cargo +1.96.0 test --release --locked \
  --features reference-simulation --test ocelot_midpoint production_execution_sweep \
  -- --ignored --nocapture
MIDPOINT_SWEEP=controlled MIDPOINT_EXECUTION_STRIDE=512 cargo +1.96.0 test \
  --release --locked --features reference-simulation --test ocelot_midpoint \
  production_execution_sweep -- --ignored --nocapture
```

All execution is local replay. No live trading or funds are used. These tests do
not establish activation acceptance, signing-delay reliability or correctness of
Titan's production route-minimum implementation.

## Remaining limits and activation requirements

- **IOC follow-up:** [book-based limits](IOC_LIMITS.md) add a 25 bp cushion
  beyond required depth, capped 10 bp inside the program policy. This addresses
  the reproduced one-tick policy-edge rejection, without predicting the mark.
  Activation remains pending. Larger movements, changed mint lot sizing,
  expired liquidity and final-output slippage can still reject execution.
- Titan must confirm recovery from simulated risk-check failures and accept the
  measured availability trade-off. The prior statement that Titan simulates
  quotes does not by itself establish retry/re-route behavior.
- Titan's production CPI allowlist must support V4 redemption, and its final
  minimum/rollback and Clock/coherent-cache delivery must be verified.
- The existing pinned oracle validity policy and special-price restrictions
  remain. The forecast-only oracle-weight restriction is gone; this does not
  establish support for arbitrary Phoenix policies. ANT is not validated.
- Negative funded collateral still disables the redemption curve. Very small
  trades can have large percentage underquotes from the fixed rounding reserve.
- Tests use the same pinned deployed Ocelot/Phoenix executables described in
  [the upgrade record](PROGRAM_UPGRADE_2026_11_17.md). Future program upgrades need
  renewed compatibility validation. No live activation is claimed.
