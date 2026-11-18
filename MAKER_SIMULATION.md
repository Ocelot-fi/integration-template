# Declared maker depth and route simulation, 18 November 2026

The adapter now quotes declared Phoenix spline liquidity without reconstructing
counterparty makers' cross-market solvency. Titan must simulate selected routes
and inspect their actual output before presenting/building the executable route.
The production adapter adds no VM or RPC dependency.

## The live failure

The original 48-account capture at slot **471112973** has a normal SOL book:
bid 7325, ask 7329 and 3,675,752 declared lots on each side. Its only maker with
live spline depth also holds positions in markets with different oracle policies.
The old native maker-risk estimator excluded that entire maker, leaving zero
prepared depth. Both SOL3L and SOL2S returned `insufficient declared spline depth`
for $1, $10, $100, $500 and $1,000 mints, although all ten executed in the reference
VM against those exact accounts. This error came from local depth lookup, not an
on-chain rejection.

The differing other-market policy has hard-stale multipliers 10 instead of zero
(metadata offsets 811 and 812). No SOL policy substitution is needed: those
other markets are no longer used to decide SOL maker liquidity.

Fixture `tests/fixtures/sol-normal-471112973.bin.gz` is unmodified public-chain
state from one `getMultipleAccounts` response. Its adjacent JSON records capture
identity. Archive SHA-256:
`f0e6812985d43807a89de2fc47a4597b8a9427079e88c61382f7029666a1bc0d`.
It includes Ocelot payload `bae5099f…` (deployed ELF `3bb3a7bb…`) and Phoenix
payload `302004ae…`, newer than the earlier slot-470782801 fixture's `fc9c3259…`.
That executable difference is recorded, not asserted to have caused the incident.

## What changes

`NativePhoenix` retains declared live/unfilled levels, maker membership and
permissions, explicit max-long/max-short position limits, and FIFO uncross
reserves. It removes estimated maker collateral, cross-market PnL/funding and
margin sums, collateral haircuts and the inferred first-tier capacity limit.
Only Ocelot's traded asset is parsed for oracle validity and risk metadata.

Curve construction, lot-boundary smoothing, marginal-price calculation, worst-mid
pricing and the 25 bp book cushion / 40 bp operating cap are unchanged. Existing
issuer fee, margin, finance, withdrawal, expiry and coherent-cache checks remain.
The adapter's negative-funded-collateral redemption restriction remains; this
change concerns counterparties, not Ocelot's own collateral. ANT oracle/special
price support still needs separate validation.

More included depth can change numerical quotes and book-based instruction limits.
Compared with public PR head `6d299b47`, the 64,709-record historical regression
changes only larger SOL3L redemptions in `sol3l-467430913`: 4,962 quote records,
from 591.592378 to 3,448.654926 tokens, improve by at most **4.189123 bp**. All
other quotes and all bounds stay identical. Seventy-four sampled instructions
change their book-derived price limit. The instruction ABI and cushion policy
are unchanged. All 4,962 distinct changed amounts were separately executed
against the deployed Ocelot ELF on that historical state, with no overquotes or
reverts. This is a local exhaustive comparison of those changed regression
amounts, not a proof over every possible maker portfolio.

Current accepted-output/price/bounds regression SHA-256:
`cb411629af7047bb7f8f8dbdac120ab5ee85ee26cf2acc0a6afd33936a889978`.
Combined quote/instruction regression SHA-256:
`cc39685074a5da9bb76bbbf0fc238692c4ead6844977ae538f6636308a91ad2d`.
Previous hashes remain in the regression test and earlier deployment record.

## Simulation contract

A quote is conservative relative to the declared-book model. **It does not
certify that every included maker can fill.** Phoenix can reject a trade or skip
an unfundable maker and use more expensive liquidity, yielding less than quoted.
A successful zero-minimum simulation therefore does not validate the estimate.

Titan should inspect actual simulated route output, reject/reprice/re-route
shortfalls, and build the user's minimum from the accepted route estimate and
selected slippage. Keep the final route minimum enforced on chain; re-simulating
cannot guarantee unchanged state during signing. Zero-minimum Ocelot legs are
only supported inside such a guarded route. This is an explicit host integration
requirement, not newly implemented Titan server behavior.

The synthetic regression changes only the live maker's settled collateral to a
$1 billion deficit, leaving its book and all native quotes unchanged. All four
small mint/redeem simulations still execute with a permissive minimum. Two
outputs are below the estimate: SOL3L mint 1,818,585 versus 1,830,254 atoms, and
SOL2S redeem 1,469,671 versus 1,477,595. An exact-quote or 0.5% minimum rejects
both with `MinimumOutput`; a 2% minimum accepts. The other directions execute
above their estimate. These deliberately extreme controls establish the need
to check simulation output, not a live failure rate or a slippage recommendation.

## Validation

- The original incident capture executes **254/254** sampled mint/redeem
  instructions across both SOL markets, including both bounds and the ten
  $1–$1,000 mints above. No accepted output exceeds execution. Maximum underquote
  for inputs at least one token is 3.361130 bp. At the tiny SOL2S redemption
  lower bound (320 token atoms), the quote is 14 USDC atoms versus 18 actual;
  percentage error is therefore 22.22%. This is unchanged integer/curve behavior,
  not an added output haircut.
- Other-market policy/staleness changes leave SOL quote output, marginal prices
  and bounds unchanged; changing the traded asset's policy still invalidates it.
- Maker-insolvency controls explicitly check worse successful fills and output
  minima. Prior tests asserting local collateral/haircut exclusion are replaced
  with the new contract; explicit position-cap execution tests remain strict.
- Existing curve, oracle, cache, withdrawal, IOC and deployed-program suites pass,
  including 1,464 deployed-program range executions and the proportional-close
  regressions. The prior golden is updated only after the independent comparison
  and 4,962 executions described above.
- Reference checks: 95 substantive tests pass. Sixteen RPC-gated shared example/
  venue tests return early without RPC and are excluded; ten opt-in tests are
  initially ignored. Twenty-seven selected default-feature checks pass.
- Constructed 0–32 tick crossing sweep: 2,681,742 curve samples and 51,706
  accepted execution comparisons, zero overquotes/reverts/range holes. The two
  previously pinned unavailable directions and two full-exit declines remain.
  Fifty-four other full exits execute. This sweep retains all curve/MVT checks;
  it is not the synthetic insolvency control above.
- Local template-router checks pass, including 12 deployed-program routed legs
  and the existing final-output-guard model: 144 successful transactions and
  96 atomic rollbacks. This guard models route protection; it is not Titan's
  production router, whose minimum handling remains Titan's responsibility.
- Four historical normal/locked/crossed timing cases: quote batch-mean medians
  0.153–0.156 microseconds, batch-mean p99 below 0.175 microseconds; refresh
  medians 1.17–1.56 ms and p99 below 1.75 ms (100 batches per case). These are
  local timings, not a production latency SLA or individual-quote p99.
- Library Clippy with warnings denied, formatting and whitespace checks pass.

Run from this package:

```sh
cargo +1.98.1 test --locked --release --features reference-simulation --lib --tests
cargo +1.98.1 test --locked --release --features reference-simulation --test ocelot_maker_simulation -- --nocapture
```

This is an adapter update, not a program deployment or routing activation. Titan
still needs to review simulation-output handling, enable the production V4 route
and verify its own final-minimum rollback. Captures and constructed tests do not
measure a long-running live rejection rate.
