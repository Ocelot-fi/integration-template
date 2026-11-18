# November 17 program compatibility update

**18 November: [verified against the deployed program](DEPLOYED_PROGRAM_2026_11_18.md).**
Mainnet ELF `3bb3a7bb…` includes proportional redemptions, separation of user/keeper
price policy and a governance mint ceiling. The adapter reads that ceiling while
preserving its 40 bp operating cap, 25 bp book cushion and existing output curves.
Captured-state execution and shifted-oracle regressions pass. Titan activation
remains pending; this verification does not deploy or activate anything.

The following records the earlier implementation and validation checkpoints.

> Historical checkpoint: this report describes the initial upgrade candidate.
> The later [current-state quoting change](CURRENT_STATE_QUOTING.md) removes its
> mark forecast and recursive redemption prefix. The fixed-through-order EMA
> premise below was disproved by review. Its zero-failure results describe the
> old restricted ranges, not the current candidate. Executable hashes and V4
> accounting remain applicable.

This candidate targets the Ocelot release deployed at finalized slot 470574016.
It changes both the host adapter and the template's on-chain CPI allowlist.
Titan must carry the V4 discriminator into its production router before enabling
this candidate. No Ocelot deployment or routing activation is part of this patch.

## What changed

- Redemption now calls `phoenix_redeem_v4`. V3 redemption is retired on chain.
  V3 mint remains active. The outer Titan `SwapRouteV3` enum and account ordering
  do not change; that router version is unrelated to Ocelot's redemption version.
- The native V4 planner uses cash first and closes the minimum whole lots needed
  for retained pool/venue leverage, minimum equity and realizable collateral.
  The program recomputes that quantity on execution; the adapter supplies the
  signed IOC price, side and reduce-only template, including zero-close exits.
- Redemption payout, allocated streaming fees, withdrawal capacity and retained
  risk follow V4. Valid full exits remain independently quotable. The router
  receives a contiguous conservative interval, which can exclude otherwise
  executable larger exits and separate valid regions.
- Mint shares use the smaller of contributed settled collateral and contributed
  equity at the captured mark. Favorable fill-versus-mark value cannot increase
  issuance above the collateral contribution. Sell-mint fees are reserved at an
  upper possible sale price: better actual proceeds can carry larger fees.
- Both instructions use the stricter of the finance policy's regular IOC price
  bound and the adapter's existing 100 bp band. This is separate from user
  slippage. Titan still enforces the final route minimum; standalone construction
  still requires a positive minimum. The deadline remains `i64::MAX`.
- Post-trade finance checks cover the supported oracle-component mark interval,
  rather than assuming Phoenix leaves the captured mark unchanged.

Worst-mid preparation remains highest live mid for buys / lowest for sells on
locked/crossed books. There is no extra five-bp haircut and no production VM or
RPC call in `quote`. Normal and worst-mid paths use the revised issuer curves.
Earlier claims that normal quotes are unchanged refer to the November 16 patch,
not this compatibility update.

## Curve and range construction

Mint output is the lower envelope of both the equity contribution bound and the
settled-collateral contribution bound, converted to shares. Fee-only sell costs
use an upper original-book price bound even when worst-mid sale proceeds are
lower. A 32-tick constructed crossing exposed why this matters: the initial
upgrade estimator quoted 3,881,407,094 atoms against 3,881,406,060 actually issued.
That case is now a mandatory execution regression, with no tolerated overquote.

For partial V4 exits, favorable execution is not credited above the gross claim.
With nonnegative funded collateral and positive holder net equity E, the planner
closes at most B*x/S + 1 + B/E lots, where B is the initial absolute position,
x the redeemed shares and S the total shares. B/E is unnecessary when fee
liability is zero. This bounds separately rounded gross/fee claims, cash-first
withdrawals, leverage bands and the minimum-equity rule. The positive execution
cost envelope at that upper quantity gives an affine output lower bound. Negative
funded collateral requires an additional realization bound and currently disables
that redemption curve.

The curve stays monotone and concave and reports its active segment's slope.
Exact whole-lot eligibility checks remain in every quote. Mint ranges intersect
linear finance constraints for all prepared lines and mark endpoints. Redemption
ranges recursively certify intervals using worst-case rounded claims, close
counts, execution cost and retained risk; unproved intervals subdivide. The 4,096
node budget returns only the already-certified prefix. It does not admit an
interval based on sparse samples. Range discovery can therefore be conservative.
Titan must honor `bounds()`.

At tiny redemption inputs the fixed rounding/lot reserve can consume nearly all
of the quote: measured relative underquotes approach 100% for sub-cent inputs.
This is not a blanket percentage haircut. Prior 12/24 bp headlines are not limits
for this release. Quote-quality acceptance must use trade sizes relevant to the
product and the newly advertised ranges, rather than those historical figures.

## Superseded post-trade mark profile

The added bound uses the oracle-anchored weighted-median model: spot plus perp
weight must strictly exceed book weight. Eligible oracle observations and the
spot-versus-mid EMA bound the possible recomputed mark; the captured mark is also
included. The existing pinned per-action validity policy, nondivergence, freshness
and special-price restrictions remain. Book-dominant/equal-weight configurations
fail refresh; ANT is still outside the validated profile.

The public Phoenix SDK supplies field layouts, not the mark evaluator. The
weighted-median / fixed-through-order EMA premise is supported by execution
probes against the pinned Phoenix binary, including 240 accepted executions with
changed weights, EMA and oracle prices. It is not a proof of arbitrary Phoenix
policies or future upgrades. Phoenix confirmation of this premise and review of
this new risk bound should accompany Titan's engineering re-review.

## Executables and fixtures

| Artifact | SHA-256 |
| --- | --- |
| Deployed Ocelot ELF, 1,589,336 bytes | `6acccee270bf0892e08e4366b53edf22814b595256c81f819bb9f1f0c4e505d8` |
| Rebuilt template router ELF | `6af1147c0a52e823daeae02683952ebf7400a9a3397b577f1c0a8f82f5374d31` |
| Fresh snapshot archive | `6180a96161a78035c4c664ddb0e4e2c1f120eb8ca7eea6a3135be6406c151563` |
| Compressed Ocelot ELF fixture | `56348aaace3c3b40765ad591350c6429184563426cd4a8857618797b4db30553` |

`sol-normal-470602378.bin.gz` is a new unmodified coherent 48-account mainnet
capture including both SOL token markets and their actual executables. Its
metadata records context/Clock slot 470602378 and all three executable hashes.
No signing keys, transaction submission or funds are needed to replay it.

Historical fixture archives remain byte-identical. `Snapshot::current` replaces
only their Ocelot ProgramData executable payload with the hash-pinned release in
memory. These runs are historical state with current code, not new live captures.
The upgrade tests also execute the fresh snapshot without replacing its bytes.
The old deployed-code validation reports are preserved as historical evidence.

## Validation and reproducibility

- 85 selected reference-enabled host tests pass, including 165 routed executions
  with exact direct/routed output equality. These are explicit local executions,
  not RPC-dependent tests reported as passed after skipping.
- 51 selected default-feature tests pass. Host Clippy passes with and without
  reference simulation, with only the upstream large-error lint exempted;
  formatting and diff whitespace checks pass.
- 37 base upgrade comparisons plus 240 altered-oracle executions pass. The fresh
  mainnet fixture is exercised in both directions. The narrower finance-price
  test adds four executions.
- 40 unchanged shared shape/construction assertions pass across eight fixture/
  market combinations; 3 router host/parity tests pass. The shared original VM
  tests remain excluded because that runtime cannot load current Phoenix; the
  compatible reference runtime supplies execution evidence instead.
- Captured + constructed worst-mid sweeps check 1,503,570 quote/shape inputs,
  12,502 sampled executions and 60 full exits, through 32-tick crossings, with
  zero overquotes, failed accepted executions or holes in advertised ranges.
  Execution sampling uses stride 512 plus endpoints, adjacent lot edges and
  seeded sizes; it is not exhaustive execution of every quote.
- Worst sampled underquote for inputs of at least 1,000,000 atoms (one input
  token, not necessarily one USDC) is 24.261 bp on locked/crossed captures and
  165.356 bp on constructed crossings. Including dust it reaches 99.861%.
  These figures are observations, not guarantees.
- The local final-output guard model gives 144 successful transactions and 96
  atomic rollbacks after completed Ocelot/Phoenix execution. Deadline controls
  give 32 direct/routed successes after +21 seconds and 16 expired rejections.
- The intentionally changed normal quote/instruction baseline has 64,720 records,
  SHA-256 `a7bec29e5ba5364724678e0bf0dbb56b4115f170ee96adba7f1d33e50d419cb5`.
  The old baseline is retained in the test's historical comment.

Local release benchmark: quote batch-mean p50 150–165 ns, p99 159–174 ns;
refresh p50 1.30–1.87 ms, p99 1.44–2.13 ms. Both bounds together take p50
34–1,572 microseconds (slowest crossed SOL2S case), separate from quote and
refresh. These are local batch measurements, not per-call tail guarantees.

The fresh capture advertises mint input ranges of 0.241536–1,000 USDC (SOL3L)
and 0.362240–3,988.686784 USDC (SOL2S); redemption prefixes are
0.000704–318.027904 SOL3L and 0.000192–0.954197 SOL2S. Individual larger exits
may execute but remain outside these conservative continuous routing prefixes.
Titan should review this availability cost explicitly.

The suite checks V3 retirement, V4/native planner quantity equality, program-side
replacement of stale signed quantities, mark containment, current mint issuance,
cash-first payouts, narrower finance price policy, full exits/bootstrap, cached
state coherence, stale oracles, withdrawal throttles, curve shape, route parity,
and output/revision/deadline rejection. Atomicity tests retain the local test-only
final-output guard; this is not verification of Titan's production guard.

Host tests use Rust 1.98.1; release benchmarks and Clippy use Rust 1.96.0.
The template router is built using `cargo-build-sbf --tools-version v1.54 --arch v0`.

```sh
cargo +1.98.1 test --locked --features reference-simulation --lib \
  --test ocelot --test ocelot_curve --test ocelot_full_exit \
  --test ocelot_midpoint --test ocelot_native --test ocelot_phoenix \
  --test ocelot_quote_regression --test ocelot_replay --test ocelot_rpc_snapshot \
  --test ocelot_slippage --test ocelot_spline --test ocelot_state \
  --test ocelot_upgrade --test rpc_cache --test your_venue_creation -- \
  --include-ignored --skip benchmark --skip fresh_spline_comparison \
  --skip production_execution_sweep --nocapture
MIDPOINT_EXECUTION_STRIDE=512 cargo +1.96.0 test --release --locked \
  --features reference-simulation --test ocelot_midpoint production_execution_sweep \
  -- --ignored --nocapture
MIDPOINT_SWEEP=controlled MIDPOINT_EXECUTION_STRIDE=512 cargo +1.96.0 test \
  --release --locked --features reference-simulation --test ocelot_midpoint \
  production_execution_sweep -- --ignored --nocapture
cargo +1.96.0 test --release --locked --features reference-simulation \
  --test ocelot_midpoint benchmark_midpoint_quote_and_refresh -- --ignored --nocapture
```

These are same-snapshot tests. Later price/liquidity changes, wallet signing time,
finance revisions and production host delivery still require Titan's usual
refresh/re-simulation and final route minimum. Account layouts and MM/rewards
entrypoints do not require adding those optional features to these user routes.
The host does not automatically recognize future program upgrades: repeat this
compatibility gate for any executable or pricing-policy change before activation.
