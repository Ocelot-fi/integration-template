# ANT quoting and Phoenix eligibility, 19 November 2026

ANT3L and ANT2S now prepare quotes from the captured Phoenix mark and declared
book. The adapter no longer requires SOL's oracle-policy hash, zero commodity /
session flags, or a zero finalized-price field. It also no longer duplicates
Phoenix's component freshness, quorum or divergence eligibility rules. Titan's
route simulation evaluates those rules by executing the actual programs.

This extends the existing [simulation contract](MAKER_SIMULATION.md): inspect
actual simulated output as well as transaction success; reject, reprice or
reroute shortfalls. Retain the user's final on-chain route minimum for landing.
An accepted native quote is an estimate against the captured mark and declared
liquidity, not a guarantee that Phoenix will execute it at that output.

## Why ANT was refused

Published source `f3b86405c` (public export `d221bde4`) still restricted the traded
asset to SOL's reviewed policy even after removing makers' cross-market checks.
ANT asset 110 has hard-stale multiplier bytes 811 and 812 equal to 10, rather than
SOL's zeroes, changing the policy hash from `afd96755…` to `7b047ebd…`. Its flags
byte at entry offset 1456 is 1. In the pinned SDK that sets `is_commodity`; it
does not by itself set the after-hours bit. Neither difference is a reason to
discard an otherwise usable captured price. The old combined error,
`stale native risk state`, incorrectly described these configuration refusals
as stale state.

No replacement policy whitelist or ANT-specific pricing model was added.
Positive, recent captured marks, account ownership/layout/binding checks,
Ocelot accounting and limits, explicit maker position caps, spline expiry,
self-trade exclusion and withdrawal budgets remain. Malformed, missing, future
or stale aggregate marks still invalidate preparation, with distinct errors.
The output curve, marginal price, worst-mid adjustment and signed IOC limits
are unchanged. No production VM, RPC call or program upgrade was added.

## Original captured state

- ANT3L market: `436HSbzkKcv5h6k4R5yDXP6vktvwYKHt7yy3gJPU1rtj`;
  mint: `4MzwrrpozKizGaBWDVEpVwNvMw5LT8g9jyeoqrWKKkHW`.
- ANT2S market: `ELVLD8SqR3e4V41f5q38bKqMj8tPZyiSz1AFLNgCkysj`;
  mint: `4g6pH8mykcikmLCdbrTUqLUEHSFqNB7CBdYLRUS1xrXa`.
- Both use Phoenix asset 110. Fixture
  `tests/fixtures/ant-normal-471413600.bin.gz` contains 48 accounts from one
  mainnet `getMultipleAccounts` response at context/clock slot 471413600.
  Original account and executable bytes are preserved. Bid 10043, ask 10072,
  captured mark 10058; the actual commodity flag is 1.
- Archive SHA-256:
  `f44112b4693a927d6f0af7ffb075e4ac5583e8e32c5401561eefd1643fa81d17`.
- Ocelot ProgramData payload SHA-256:
  `bae5099f673264794a1e8cbea540b75e9f229aa51b7d7cb4dfcf432685e59ca5`.
  This includes allocation padding; its deployed ELF is the unchanged
  `3bb3a7bb74aee8fb84e831f124ad798df97d39305f9ca60106886ad3bee1a9e6`.
- Phoenix ProgramData payload SHA-256:
  `302004ae6d89b08f905ecda265d3fca77ed43ab30475396c150b3b70b91162f4`.
  The adjacent fixture JSON records all program payload identities.

## Validation

`tests/ocelot_ant.rs` covers both markets and directions:

- 258 built-instruction executions across the advertised ranges, including
  $1, $10, $100, $500, $1,000 and $5,000 mints and full-supply redemptions.
  Every execution succeeds, spends the complete input and returns at least
  the quoted output. Each instruction's minimum is the native quoted output.
- A $1,000 mint for each market, followed by refreshing from the actual VM
  post-trade accounts and redeeming a quarter, half or all newly issued tokens:
  eight successful executions, no overquotes.
- 154,651 lot-edge/interior quotes: no range holes, output decreases or
  marginal-price/curve-shape failures under the shared suite's rounding tolerance.
- 20 executions with controlled commodity/session flag combinations preserve
  quotes and execute. These are mutations of one snapshot, not independent
  after-hours captures. A zero aggregate-mark timestamp still invalidates state.
- 12 successful trades through the local template CPI router. This verifies
  the supplied router artifact, not Titan's production activation.

The broader reference suite passes, including unchanged SOL quote/instruction
regression hashes, deployed-program execution, curve, IOC, cache, maker and
slippage checks. Updated oracle tests explicitly show that native estimates
remain available when Phoenix simulation rejects a stale/divergent policy;
they do not assert that every estimate executes. Selected default-feature tests,
library Clippy and formatting pass. RPC-gated shared tests without credentials
are not counted as live integration acceptance.

Run the ANT checks from the standalone crate (or add its manifest path):

```sh
cargo +1.98.1 test --locked --release --features reference-simulation --test ocelot_ant
# Requires the existing built program-template SBF artifact:
cargo +1.98.1 test --locked --release --features reference-simulation --test ocelot_ant -- --ignored
```

This is captured-state execution evidence, not a guarantee about future session
transitions, maker solvency or signing delays. Those are evaluated by simulation
and the on-chain route minimum. Negative-funded-collateral redemption behavior
is unchanged. Titan must update its imported adapter to this revision; publishing
this PR does not replace its internal `ocelot 0.2.1` package automatically.
