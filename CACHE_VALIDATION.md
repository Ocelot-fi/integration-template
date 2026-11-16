# Coherent account refresh, 16 November 2026

## Strict account decoding

The tooling RPC helper now reads typed `UiAccount` responses and decodes every
non-null account explicitly before publishing a snapshot. Only an actual RPC
`null` becomes `None`; invalid base64, corrupt/truncated zstd or an invalid owner
reject the entire batch. A failed decode leaves the previous published snapshot
and accepted slot floor untouched. Single-account misses use the same path.
This closes the SDK convenience method's ambiguity between missing accounts and
decode failures, including optional market withdrawal-limit accounts.

Three regressions cover real compressed bytes plus null/duplicate/unknown keys,
malformed account responses without partial publication or slot advancement,
and Ocelot refresh through the actual RPC cache with a corrupt optional throttle.
The latter clears quote/bounds/instruction state and recovers when RPC actually
returns null. The corrupt-batch and optional-throttle tests both fail before the
fix: the previous helper accepts their corrupt accounts as absent.

`tests/common/mod.rs` contains the shared simulation preparation changes from the
previous update. Its acceptance assertions remain unchanged; the `your_venue`
header now explicitly distinguishes the two. The host cache contract also states
that unreadable data must error, since absence has meaning for optional accounts.

The first full live run also exposed a separate routed-test harness still doing
fresh per-sample RPC reads (`program-template/.../tests/common/mod.rs`). After
several successful comparisons its RPC node rejected a read with error -32016
(minimum context slot not reached). Code inspection also confirmed those reads
could mix quote/simulation state. The route harness now captures all quote,
instruction and clock dependencies once, rebuilds from that snapshot and requires
`RpcSnapshot` in its simulation loader. Exact-equality assertions are unchanged;
no RPC reads occur during its comparison loop. The nested lockfile now records
the decoder dependency already used by the host crate; no dependency versions
or on-chain source changed.

Pricing, five-bp midpoint adjustments, instruction bytes and on-chain programs
are unchanged. No retry or scheduling redesign is included.

Validation: 22 focused reference-feature tests and 12 default-feature tests pass,
including the frozen 64,716-record quote/instruction baseline. Both selected
Clippy configurations, formatting and shared test compilation pass. Valid zstd
bytes are exercised rather than only mocked plain-base64 responses.

The complete live `make test-example` rerun passes without skips against the
public Solana mainnet RPC: eight reference tests, two creation/parser tests and
the routed test with 20 exact-output comparisons across both directions. The
first run's provider -32016 error is retained in local evidence; snapshot pinning
removes per-sample network reads but does not guarantee that initial capture can
never hit provider lag. No retries or relaxed slot floor were added.

The existing router artifact matches the previously recorded SHA-256
`b8997ff5ef34d8fa27a1e71fdfa1b134a6938788de920fa0e1d19b1f7d8652ba`;
it was reused, not rebuilt or deployed by this change. These are local VM
executions using live-read state, not transactions submitted to mainnet or a new
Ocelot live refusal-rate measurement. Titan's sampler cache confirmation remains
open.

## Review follow-up: pinned simulations and complete discovery

The first cache fix changed a shared RPC helper without pinning the shared
simulation harness. Later simulation reads could therefore load newer accounts
than the venue's quote. This follow-up fixes that compatibility regression rather
than weakening quote/execution equality assertions.

- `RpcClientCache::snapshot` captures an immutable bank response. Unknown keys
  error, while accounts known absent in that response return `None`. Snapshot
  reads never fetch RPC data. The shared boundary/random simulation tests gather
  quote and instruction dependencies, capture them with the clock, reconstruct
  the venue from the pinned pool account and refresh it from that same snapshot.
  Both the simulation loader and clock helper now require `RpcSnapshot` by type.
- RPC refreshes retain `context.slot`, pass `min_context_slot` and reject backwards
  responses even if a provider ignores that parameter. A serialized fetch path
  and atomic replacement of the latest snapshot prevent concurrent publication
  from assembling mixed generations. Single misses use the same path. Resetting
  cached accounts preserves the slot floor. Explicit base64+zstd encoding retains
  the SDK's previous behavior for large accounts. Individual calls may still advance
  between snapshots; callers needing repeatable reads must retain a snapshot.
- Ocelot discovers both directions' account metas independently of trade sizing.
  A synthetic instruction failing to size can no longer silently omit keys.
  Actual instruction construction uses the same extracted account layout.
- Required-key errors are propagated only after prepared state is cleared. Warm
  dependency keys are still collected before clearing, preserving subscriptions.

### Follow-up validation

- 78 selected reference-feature tests and 48 default-feature tests pass; four
  default benchmark tests remain ignored. Both selected Clippy configurations
  and formatting pass. The frozen 64,716-record quote/instruction baseline and
  existing normal/midpoint pricing assertions remain unchanged.
- 40 shared quote-shape/construction assertions pass against eight fixture/market
  combinations. Shared simulation preparation changed; assertion bodies did not.
- The end-to-end regression runs `OcelotVenue::update_state` through the actual
  RPC cache and an in-process sender advancing its bank slot on every request.
  A negative control applying the original hit-reuse policy reproduces `invalid
  withdrawal budget`; fresh cold/warm refreshes succeed with matching clocks and
  withdrawal queue timestamps. Makers refresh unchanged prices in this mock so
  liquidity expiry does not confound the cache test.
- The same test invokes the actual shared-suite snapshot helper, advances the
  live cache again, and verifies pinned account/clock reads, unchanged quotes,
  two successful actual-program comparisons, and no additional RPC requests.
- A controlled index-node relocation with matching buffer references constructs
  coherent before/after maps. Mixing the old index with the new buffer triggers
  `orphan active position`, clears quotes, and then recovers on the coherent map
  with unchanged output. This is parser/refresh regression coverage, not a claim
  that the constructed relocation represents a particular live Phoenix event.
- RPC tests verify pinned absence versus unknown keys, nondecreasing slots across
  cache reset, ordered/duplicate requests, transport/size failures, and concurrent
  refreshes publishing whole snapshots with no overlapping RPC fetches.
- The live Raydium suite was compiled but not executed: no `SOLANA_RPC_URL` or
  Raydium program dumps are available. The offline test exercises its actual
  generic pinning helper, but is not a live Raydium execution result. A live test
  must retain the pinned snapshot; passing once with moving reads is insufficient.

### Remaining partner confirmation

Titan has not yet confirmed whether its 300-call sampler used the template's
`RpcClientCache` or another cache implementation. These changes fix the bundled
helper and shared harness, and document the production-host coherence contract.
They do not modify Titan's proprietary cache or establish a new live refusal rate.
Ask which cache was used and request a rerun on this commit; do not describe the
reported production incident as independently confirmed resolved.

Pricing, the common-midpoint/five-bp policy, withdrawal guards, transaction bytes
and on-chain programs are unchanged. Worst-mid remains a separate proposal.

## Initial cache fix (historical; tooling details superseded above)

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
