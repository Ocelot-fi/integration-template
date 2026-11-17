# Proportional redemption candidate, 17 November 2026

**This adapter targets an un-deployed Ocelot program correction. Coordinate the program,
SDK and Titan rollout before activation.** It is not the executable currently on mainnet.
V4's instruction/account ABI is unchanged; the program reconstructs the order quantity.

## What changes

The earlier V4 planner closed only enough exposure to stay inside its leverage ceilings.
That could leave remaining holders more leveraged and make an ordinary execution-time
Phoenix mark refresh fail the retained-risk check. Cash could permit a zero-lot close.

The corrected planner closes at least `ceil(abs(base) * shares / total_shares)` lots,
regardless of cash or band headroom. It also preserves the retained net-backing fraction
after separately floored gross/fee claims. Existing stricter collateral, unrealized-PnL
and minimum-equity requirements may close more. Abandoned claims remain in the denominator.

`quote.rs` mirrors that program change. The conservative curve and worst-mid pricing
source are unchanged, with no new haircut, mark forecast, narrowed routing prefix or VM
in the production path. `quote` still uses prepared state; the VM is test-only.
Small claims still pay actual whole-lot execution costs and can be too small to execute.

A 64,709-record comparison with the preceding adapter preserves every accepted output,
marginal price and advertised bound. The normalized accepted-record hash is
`50b34b5ffae92de2f8772a6358c6d03cfe6e376fa47c0cb23c3aed3fd2137288` in both versions.
Instruction lot hints change, and three one-atom inputs still decline with a different
error message. The new full quote/instruction baseline is `c1b75fc3…`.

## Program and replay identity

- Main program patch: `50461040f95a829d27807a1196395bb7b1ccf51b`.
- Audit mirror patch: `14274a7e50fe83b10b4f0948358cc134cc236453`.
- Candidate ELF SHA256: `506ef261c9556ea0223757770c733d662d48aa7de71f6d880e8d13a30f64f7b4`
  (1,588,920 bytes); independently rebuilt twice with matching sources/artifacts.
- Candidate IDL SHA256: `948dd81e01da60e0c0dbedfb632c9029db69dde017136f2c305b206564b07d7c`.
  Only V4 documentation changes; ABI is identical.
- Deployed November 17 ELF remains separately preserved as `ocelot-mainnet-20261117.so.gz`,
  SHA256 `6acccee270bf0892e08e4366b53edf22814b595256c81f819bb9f1f0c4e505d8`.
- `Snapshot::candidate` replaces ONLY Ocelot's executable in captured accounts with
  `ocelot-proportional-candidate-20261117.so.gz`. Phoenix/Ember executables and original
  snapshot files remain unchanged. `Snapshot::read` reads the original deployed snapshot.

This is offline execution with synthetic user balances; it is not a live rollout.

## Regression evidence

`ocelot_proportional` compares deployed and candidate executables against the same SOL
snapshot. Changing one perp observation by one tick (preserving the captured EMA) reproduces
46/61 old SOL3L failures. The candidate executes 61/61. Across both SOL tokens, all-oracle
changes of -3/-1/0/+1/+3 ticks plus that single-observation case, 732/732 candidate exits
execute with the quoted amount as their enforced minimum, no overquotes and the exact
proportional quantity observed in the post-trade position. No quote declines are skipped.

Whole-supply coverage executes 1,230/1,230 sampled redemptions over four historical states
and both markets in the recent capture. Zero executable declines, accepted failures or
holes in advertised ranges. The former six historical risk failures are gone.

Locked/crossed sweeps now explicitly assert zero execution failures as well as zero
overquotes and holes; no unbounded rejection allowance remains:

| Sweep | Curve points | VM comparisons | Failures / overquotes / holes |
| --- | ---: | ---: | --- |
| Captured locked/crossed states, 8 directions, execution stride 256 | 188,782 | 1,470 | 0 / 0 / 0 |
| Constructed crossings 0–32 ticks, 112 directions, execution stride 64 | 2,705,934 | 52,262 | 0 / 0 / 0 |

Stride samples execution; all selected curve/lot-edge/chord checks still run. Very small
outputs can have large percentage underquotes because a few atoms are material. This is
sampled evidence, not a universal Phoenix slide or delayed-execution guarantee.

The general stressed finance test still pins two **mint** minimum-equity endpoint
rejections, at inputs 2,222,528/2,222,529 in `sol2s-467431351` with the constructed 120%
minimum-equity policy. Neither is a redemption. The helper accepts only known risk sites;
other errors fail the test. The old unbounded native/spline/fee/routed false-decline
exceptions were removed. All protected-route rollback checks remain.

## Integration checks and timing

The source candidate passes 77 reference host tests, 44 default host tests, one creation
parser test, three explicit routed/rollback tests, and 40 unchanged shared Titan
shape/construction checks over eight fixture/market pairs. The two execution sweeps above
and the release benchmark are additional explicit runs. Shared live-RPC simulation and
random-execution gates are not claimed as production Titan acceptance.

The final-route guard test executes 144 successful transactions and verifies 96 complete
rollbacks after Ocelot/Phoenix execution when the user's minimum is not met. The deadline
test retains 32 direct/routed successes after 21 seconds and 16 explicit-expiry rejections.
Router and guard sources/binaries are unchanged. This validates the local router model;
Titan still needs to confirm its production final-minimum enforcement.

Default release benchmark over four normal/locked/crossed fixture-market cases: quote
batch-mean p50 130–143 ns; p99 142–301 ns, below the 1 microsecond budget. Refresh p50
1.28–1.97 ms; p99 1.53–5.62 ms. This is local timing, not a production latency SLA.

```sh
cargo +1.98.1 test --locked --release --features reference-simulation --test ocelot_proportional --test ocelot_availability --test ocelot_upgrade
MIDPOINT_EXECUTION_STRIDE=256 cargo +1.98.1 test --locked --release --features reference-simulation --test ocelot_midpoint production_execution_sweep -- --ignored
MIDPOINT_SWEEP=controlled MIDPOINT_EXECUTION_STRIDE=64 cargo +1.98.1 test --locked --release --features reference-simulation --test ocelot_midpoint production_execution_sweep -- --ignored
cargo +1.98.1 test --locked --release --test ocelot_midpoint benchmark_midpoint_quote_and_refresh -- --ignored
```

## Remaining coordination

Independent program review and an explicitly authorized program upgrade are required.
Titan should review this matching candidate and its new binary rather than execute it
against the old planner. Production V4 allowlisting and Titan's own final route-minimum
rollback remain acceptance requirements. Simulate/re-route failures and refresh stale
wallet transactions; a quote is not a promise across arbitrary state changes.

The separate signed IOC limit sitting at the configured price-policy edge remains open.
This quantity fix does not add IOC headroom or relax a program risk check. ANT remains
outside the validated oracle profile. No merge, on-chain upgrade or routing activation
is included in this change.
