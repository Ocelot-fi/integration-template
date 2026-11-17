# Book-based IOC limits, 17 November 2026

Adapter follow-up to the proportional-redemption candidate for review in
[Titan PR #10](https://github.com/Titan-Pathfinder/integration-template/pull/10).
Not activated. No program, SDK, instruction ABI or output
discount changes. The program candidate still requires a separately authorized
upgrade; this adapter change does not make it compatible with the old planner.

## Policy

The old builder placed its signed limit on the configured price-policy boundary
(50 bp in the captured live policy). The program checks that limit against its
execution-time mark, even when the available fill is much better. A one-tick
mark move could therefore reject an otherwise executable order.

The adapter now uses two controls:

- Quoting and curve preparation use a shared cap with **10 bp reserved inside
  the program policy**. The deviation is `min(100, policy_bps.saturating_sub(10))`.
  At the current 50 bp policy this is a 40 bp cap. Buy caps round down to a tick;
  sell floors round up. Policies of 10 bp or less use a zero-deviation cap and
  cannot provide the full 10 bp reserve.
- Instruction construction finds the **last price needed for the full modeled
  quantity** in the prepared, maker-limited book, adds a **25 bp cushion**, and
  clips it to that cap. Buy cushions round up; sell cushions round down. A
  clipped cushion is allowed if the required fill still fits. Insufficient
  depth or a required price outside the cap rejects the instruction.

For buys this is `min(ceil(worst_ask * 1.0025), cap)`; for sells it is
`max(floor(worst_bid * 0.9975), floor)`. The price is the last required level,
not the average fill or the top of book. Locked/crossed books use the existing
side-specific worst-mid model and its prepared risk limits.

The amount-dependent lookup runs only while building an instruction. The hot
quote path retains its existing cached depth search and conservative curve.
There is no new RPC, VM call, mark forecast, or output haircut. A tighter shared
cap can reduce quotable capacity in books whose marginal liquidity lies near
the old boundary. It does not modify the lot-boundary smoothing formula.

## Execution evidence and limits

- `ocelot_ioc`: 420 actual built instructions across ten fixture/market pairs,
  mint/redeem and the full advertised ranges, including locked/crossed captures.
  All execute, with outputs identical to independent shared-cap replay and no
  overquotes. The standalone minimum is the quoted output in this comparison.
- The delayed-mark regression signs once, then changes the execution bank's
  stored mark and oracle observations by -5, -1, 0, +1 and +5 ticks. All 40
  sampled instructions execute; restoring only the old signed price limit
  reproduces 16 `FinanceExecutionBound` failures. These synthetic changes keep
  the book and clock fixed; they do not model all consequences of signing delay.
- That regression uses minimum output 1 to isolate IOC behavior. It does not
  claim the old quote still satisfies every user-selected slippage setting.
- Normal quote regression retains the existing accepted-output/price/bounds
  hash `50b34b5ffae92de2f8772a6358c6d03cfe6e376fa47c0cb23c3aed3fd2137288`.
  The combined instruction hash changes because the signed price bytes change.
- Tests cover last-level selection across multiple levels, insufficient depth,
  both sides, clipping a cushion, rejecting required prices beyond the cap,
  low-price tick rounding and policies smaller than the reserve.
- The controlled crossing sweep explicitly pins two full exits excluded at
  artificial 32-tick crossings: SOL3L `467430913`, lowering the selected maker,
  and SOL2S `467431351`, raising it. The first still executes in independent
  shared-cap replay (a conservative model decline); the second cannot fill even
  with that cap. No accepted-quote execution failure is waived. The separate
  full-exit capacity test also pins unavailable mint depth in its 32-tick case.
  Final sweep: 112 direction/state combinations, 2,649,409 curve checks and
  30,585 actual generated-instruction executions, zero overquotes, accepted
  execution failures or advertised-range holes. Two entire directions decline
  at artificial 32-tick crossings; remaining supported ranges are checked.
- Default-feature checks, the shared Titan curve/construction tests, and the
  router replay/output-minimum rollback tests pass. The output guard is the
  existing local test model, not verification of Titan's production router.
- Local release timing across normal/locked/crossed samples: median batch-mean
  quote time 124–141 ns, p99 batch means 136–158 ns; median refresh 1.32–1.92 ms.
  These are host measurements, not individual-quote tail latency guarantees.
- Validation: 79 selected reference-feature unit/integration/parser tests,
  47 default-feature checks, 20 shared Titan checks and three explicitly run
  routed/rollback tests pass. Library Clippy with warnings denied, formatting
  and whitespace checks pass. The two pinned constructed mint minimum-equity
  endpoint rejections from the prior candidate remain unchanged.

Focused reproduction from this package:

```sh
cargo +1.98.1 test --locked --release --features reference-simulation --test ocelot_ioc -- --nocapture
MIDPOINT_SWEEP=controlled MIDPOINT_EXECUTION_STRIDE=128 cargo +1.98.1 test --locked --release --features reference-simulation --test ocelot_midpoint production_execution_sweep -- --ignored --nocapture
```

`reference_quote()` deliberately retains the shared-cap instruction without
consulting native modeled depth. This preserves an independent execution check
for false declines. Actual book-cushioned instructions are executed separately
by `ocelot_ioc`, the upgrade/proportional tests and the midpoint execution sweep.

Price headroom is finite. Liquidity can move or expire; the reference mark can
move beyond the reserve; final output can fail user slippage. The mint lot count
is still signed and can become invalid as the mark changes, especially at the
one-lot minimum or for larger mints. V4 recalculates redemption quantity but
retains the signed price. Those pre-existing execution constraints remain;
Titan must simulate and rebuild/re-route failed transactions. No broad
quote-to-landing execution guarantee is claimed.

Before release, review these adapter settings with Titan and coordinate the
program candidate, V4 allowlist and route-minimum verification. This adapter
change does not authorize activation.
