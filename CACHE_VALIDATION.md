# Coherent account refresh, 16 November 2026

This follow-up addresses mixed account versions during discovery and in the
bundled RPC-backed cache. It does not change quote curves, midpoint selection,
the five-bp output discount, finance/withdrawal guards or transaction instructions.
Worst-mid pricing remains a separate proposed change. No on-chain program or
routing activation is included.

## Report and cause

Titan reported three `orphan active position` refresh refusals in 300 calls, with
the Phoenix index and active buffer fetched one slot apart. Batching those three
discovery dependencies eliminated the error in their next 300-call sample, but
one refresh then saw a withdrawal queue ahead of its clock. These are partner
observations, not a live reproduction by this change.

The adapter already discarded discovery data and made a final complete batch
read. However, the template's `RpcClientCache::get_accounts` reused earlier cache
hits and fetched only misses. A nominal final batch could therefore retain
accounts from earlier discovery reads. Existing tests assumed coherent batches;
they did not exercise this behavior in the actual RPC-backed cache.

## Change

- No individual dependency reads in `update_state`. After resolving dependency
  addresses, replace the discovery map with a full batch containing the trader,
  index, active buffer, withdrawal queue, clock and original configuration keys.
  Cold discovery uses three batch calls; ordinary warm refreshes still use two.
- The final complete quote-preparation batch remains separate from discovery.
  Invalid/missing accounts still clear prepared state; no guard is relaxed.
- The template's RPC cache fetches every requested key in one RPC response, even
  when cached. It returns that response directly and updates its single-account
  cache for later callers. Deletions overwrite cached values with `None`.
- Empty requests return immediately. Requests above the standard 100-account RPC
  limit fail explicitly instead of being split into inconsistent snapshots.
  Transport errors and malformed response counts cannot fall back to old hits.
- `get_account` and the diagnostic `get_multiple` remain cache reads. They do not
  promise snapshot coherence. The optional reference simulator can still fetch
  program binaries individually; it is not used during production preparation.

Titan supplies its own production `AccountsCache`; no RPC is introduced into the
adapter or `quote()`. That host must supply a coherent final snapshot including
clock. Account last-write slots need not match when older accounts remain valid
at that snapshot. The current trait returns no slot/write-version metadata, so
the adapter cannot independently certify arbitrary host batches.

The RPC helper now spends one network request per nonempty batch instead of
serving potentially stale cache hits. It is intended for local tooling/tests;
this does not impose network requests on Titan's production local cache.

## Validation

- 74 selected reference-feature tests pass under `release-debug`, including
  deployed-program replay and route-level rollback checks; expensive benchmark,
  live/research and dense sweep tests are excluded from this selection.
- 46 selected default-feature tests pass; four benchmark tests remain ignored.
- Both selected Clippy configurations and formatting pass.
- The frozen normal-book regression matches all 64,716 records, including 487
  instructions. Normal and midpoint tests retain their existing expected prices,
  bounds and output-discount checks.
- Two new tests use the actual `RpcClientCache` and a deterministic in-process
  RPC sender. They catch earlier single-read values contaminating a final batch,
  preserve request ordering/duplicates, handle deletion/recreation, and reject
  transport/response-size/oversized-batch failures without returning stale hits.
- A new four-fixture test forbids single dependency reads, reproduces the exact
  `invalid withdrawal budget` guard with an older clock/newer queue, then verifies
  coherent refresh and eight successful mint/redeem reference executions.
- All three new regressions fail against pre-fix `147da3fc`: both RPC tests fail
  on stale-hit behavior, and cold discovery attempts a prohibited single read.
  They pass with this fix. This does not reproduce Titan's exact orphan snapshot.

Focused reproduction from the package root:

```sh
cargo +1.96.0 test --release --locked --features reference-simulation --test rpc_cache --test ocelot_state --test ocelot_quote_regression
```

Shared assertion source, program sources and SBF artifacts are unchanged. The
prior 40 shared checks and program artifact checks were not rerun for this
account-fetch-only follow-up. No new timing or live refusal-rate claim is made.
