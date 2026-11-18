# Deployed program compatibility, 18 November 2026

The adapter has been checked against the actual mainnet executable, including a
fresh, coherent capture of both SOL markets. The previous PR head already
executes the sampled current configuration. This follow-up handles the newly
configurable mint ceiling and removes the obsolete coupling to keeper price bands.
Titan activation remains a separate host/router decision.

## Verified deployment

Read-only finalized RPC verification on 18 November:

- Program: `CpLah6JAC9967YiVQnQJegfCKEjLEcUYmx3PCYgg9xWf`.
- ProgramData: `27j7YsZn9KiYU1HGoA8FDqwM27i8kRVmHm1hFEedEMDz`.
- Deployment slot: **470782007**. [Finalized upgrade transaction](https://solscan.io/tx/5GXMNm5aEc11L7aEeZzcy7AJhrpdm3UuoNrvr6Nd9CoXBkFdwQvubMvHjsFReUHmNJj1kJC86zQpaiAy8HyFA3ga).
- ELF: **1,593,280 bytes**, SHA-256
  `3bb3a7bb74aee8fb84e831f124ad798df97d39305f9ca60106886ad3bee1a9e6`.
  This matches the recorded independent main/audit build. This check downloads
  and compares the deployed bytes; it does not claim another independent build.
- Allocated executable payload: 1,599,576 bytes, SHA-256
  `bae5099f673264794a1e8cbea540b75e9f229aa51b7d7cb4dfcf432685e59ca5`.
  The extra 6,296 bytes are all zero padding after the ELF. The ProgramData header
  is another 45 bytes and is excluded from both executable hashes.
- Source baseline: main merge `705391e0289717c670862347ad886b10bd1bc6fb`.
  Program changes: proportional close `59b814ec5ca9d68f891ce3c6e99e49dbca66501f`,
  user/maintenance policy separation `cfc7ecb3e288053fba75b08b11442fcdfdd2294a`,
  mint governance `2875888a0e32775ac125f4ae0b872015fa377c95`.

`sol-normal-470782801.bin.gz` contains 48 original accounts from one
`getMultipleAccounts` response, with context slot and clock slot both 470782801.
It includes original Ocelot/Phoenix/Ember executable accounts, without mutations.
Both captured SOL Market accounts store mint ceiling zero, resolving to 300 bp.
Historical captures are unchanged; `Snapshot::deployed` explicitly substitutes
only Ocelot's payload when replaying old state against this release. The older
`Snapshot::candidate` remains pinned to `506ef261…` for historical regressions.

## Adapter delta

Mint V3 and redeem V4 instruction/account layouts are unchanged. Proportional
redemption sizing was already implemented in the preceding adapter commit.
The new program also separates user IOC limits from keeper execution policy:

- Decode Market `max_move_bps` at bytes 397..399. Zero means 300 bp;
  1..=1,000 is a configured mint ceiling; larger values disable mint quoting.
  An invalid mint ceiling does not disable redemption.
- Keep the adapter's existing **40 bp operating cap** instead of widening it to
  the new program ceiling. For mints, use
  `min(40, resolved_mint_ceiling.saturating_sub(10))`. Redemptions retain 40 bp,
  inside their separate 1,000 bp program backstop.
- Keep the **25 bp cushion beyond the worst required fill**, clipped to the
  operating cap. Both the curve and instruction builder use the same cap.
- Valid keeper price-band changes no longer change user quotes or signed limits.
  Invalid keeper policy still declines: the program validates FinanceState even
  though its keeper price band no longer bounds ordinary user execution.

At the captured 300 bp mint ceiling / 50 bp keeper policy, outputs, marginal
prices, bounds and instruction bytes are unchanged. Low mint ceilings can reduce
mint capacity; ceilings at or below 10 bp cannot retain the full 10 bp reserve.
No change to curve smoothing, worst-mid pricing, output discounts, deadlines,
route-level slippage, RPC/VM behavior, or the program itself is included.

## Validation

- Before the adapter fix, **1,464/1,464** sampled actual built instructions
  executed against the deployed ELF with no overquotes, across 12 fixture/market
  pairs: both directions, fresh state, historical normal, locked and crossed
  books. The configured-ceiling check exposed the mismatch.
- After the fix, the same **1,464** instructions still execute with no overquotes
  or reverts, and outputs equal independent shared-cap replay. Each trade checks
  actual exposure against native sizing; exits close at least the redeemed share.
- **732** shifted-oracle redemptions pass against the deployed ELF, including the
  original one-tick case. The old November 17 program still reproduces 46/61
  SOL3L failures in that control; the historical proportional candidate's 732
  cases also continue to pass.
- **21** executions cover mint ceilings 0, 1, 25, 300, 1,000 and invalid 1,001,
  long/short mints and exits. The one unavailable case is an adverse-side mint
  under the 1 bp ceiling; invalid-ceiling mints are explicitly declined.
- **16** executions verify valid keeper price policies do not alter user bounds,
  output or instructions. Invalid 0/1,000 policies still clear prepared state.
- **12** template-router legs execute against the fresh deployed capture, at
  both endpoints and midrange, with reserved router balances. This is the local
  template router, not Titan's production router.
- **79** reference-feature unit/integration checks pass (ten opt-in tests ignored
  by that invocation). The deployed suite is separately run with its router test
  enabled: all four pass. Historical IOC, curve, oracle, cache-coherence, throttle,
  full-exit and quote regressions remain in the selected suite.
- **25** default-feature checks pass. Eight RPC-gated shared tests return early
  without an RPC URL and are excluded from that count. Library Clippy with
  warnings denied, formatting and whitespace checks pass.
- The 64,709-record quote/instruction regression remains pinned: accepted
  output/price/bounds hash `50b34b5ffae92de2f8772a6358c6d03cfe6e376fa47c0cb23c3aed3fd2137288`,
  combined instruction hash `32386193f752dea354edf1d5287d90154c588347a8559dc19caf4a32ce13fe4b`.

These are local executions of captured public programs and accounts with
synthetic user balances; no mainnet trade was submitted. The new capture is one
normal-book state, not a live availability survey. Wider constructed crossing
sweeps and timing results in older records remain historical, not rerun claims.
Titan still needs to accept the host integration and enable V4 in its production
CPI allowlist, verify its own route-minimum rollback, and simulate/rebuild or
re-route failures. Liquidity expiry, changing mint lot counts and user slippage
can still invalidate a transaction during signing. ANT remains outside this
adapter's validated oracle profile.

Reproduce the deployed checks from this package (router test requires the built
local template SBF):

```sh
cargo +1.98.1 test --locked --release --features reference-simulation --test ocelot_deployed -- --include-ignored --nocapture
cargo +1.98.1 test --locked --release --features reference-simulation --test ocelot_proportional --test ocelot_ioc --test ocelot_quote_regression
```

## Artifact checksums

- `ocelot-mainnet-20261118.so.gz`: `0837fd6949880f69991f049e26e51d8655c4cebad8d58d2bc0fae6d8afd63ee9`.
- `sol-normal-470782801.bin.gz`: `eddd5cb567a148ead35bf20c3ba33063054f0d5c937b5a58711b91a52fa0f44e`.
