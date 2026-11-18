# Public-chain replay fixtures

These gzip archives contain captured public Solana accounts, including deployed
programdata, not wallets or private signing material. They are used only in a
local bank with synthetic user balances. Historical archives stay unchanged. Candidate replay uses
`Snapshot::candidate` to replace only their Ocelot executable payload in memory;
this is historical state with un-deployed candidate code. Negative tests fork state explicitly.

See [the verification record](../../HANDOFF.md)
for capture slots, executable/fixture hashes, outputs, limitations, and the
quote-curve counterexample. `examples/probe.rs` writes the same archive format.

After gzip decoding, bincode deserializes a `BTreeMap<String, Vec<u8>>`. Each
address maps to bincode-encoded `Option<solana_account::Account>` using the host
crate's pinned Solana 2 account layout. The test reader loads this in memory;
it does not extract executable files or submit transactions.

The November 14 midpoint policy adds two original coherent captures:
`sol-locked-468615258.bin.gz` (raw bid/ask 8167/8167) and
`sol-crossed-468615690.bin.gz` (8171/8170), each containing both SOL token markets.
Adjacent JSON files record account counts and executable/fixture hashes.
`sol-book-window-2026-11-12.jsonl` is the short 120-observation live window;
observation counts are not a time-weighted availability estimate. See
[current midpoint validation](../../MIDPOINT_VALIDATION.md).

The November 17 compatibility update adds `ocelot-mainnet-20261117.so.gz`
(hash-pinned deployed Ocelot release) and the unmodified coherent capture
`sol-normal-470602378.bin.gz`, with both SOL token markets. The original capture remains the deployed baseline; current candidate tests replace only
Ocelot code explicitly. See [release evidence](../../PROGRAM_UPGRADE_2026_11_17.md)
for exact hashes, current-code replay semantics, coverage and limitations.

The proportional correction adds `ocelot-proportional-candidate-20261117.so.gz`,
a separately hash-pinned, **un-deployed** program build. `ocelot_proportional` replays
both original and candidate programs against the same account snapshot to reproduce
and fix the one-tick redemption failure. See [candidate identity and evidence](../../PROPORTIONAL_REDEMPTIONS.md).

The November 18 verification adds `ocelot-mainnet-20261118.so.gz` (actual deployed
ELF `3bb3a7bb…`) and the original coherent `sol-normal-470782801.bin.gz` capture.
`Snapshot::deployed` replaces only Ocelot code for explicit historical-state
replay. The fresh capture is replayed unchanged, including ProgramData padding.
See [deployed identity, source commits and validation](../../DEPLOYED_PROGRAM_2026_11_18.md).

The later November 18 live-depth incident adds `sol-normal-471112973.bin.gz` and
its original capture JSON: 48 accounts from one RPC response, including the then
live Phoenix payload `302004ae…` and unchanged Ocelot payload `bae5099f…`.
No account or executable substitutions are used for this regression. Synthetic
maker-insolvency and other-market-policy controls clone it explicitly in tests.
See [maker simulation contract and incident evidence](../../MAKER_SIMULATION.md).
