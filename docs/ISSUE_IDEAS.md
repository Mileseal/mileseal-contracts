# Issue ideas

Not GitHub issues. A pool of genuinely distinct, real work this repo could use, found while
building and testing `contracts/escrow`. Sized honestly; the maintainer turns a subset of
these into scoped GitHub issues after Wave approval.

## Contract features

### 1. Dispute timeout
**Complexity: high**
Today, a `Disputed` milestone has no way out if the arbiter never calls `resolve` — this is
a documented known limit. Add a timeout (e.g. a new field or a fixed window past
`submitted_at + review_window`) after which either the funder can reclaim the funds, or the
milestone falls back to a default split. Must not change behavior for existing escrows
created before the field existed, if storage is extended.
**Acceptance criteria:** a disputed milestone past the timeout can be resolved by someone
other than the arbiter; before the timeout, only the arbiter can resolve it; tests cover
both sides of the boundary.

### 2. Cancel a pending milestone by mutual consent
**Complexity: medium**
Right now a milestone cannot be cancelled before its deadline even if the funder and every
payee agree to call it off. Add a `cancel(id, index)` that requires auth from the funder
and every payee of that milestone, refunds the funder, and only works while `Pending`.
**Acceptance criteria:** fails if any required party hasn't authorized; fails outside
`Pending`; refunds the full amount; emits a new event.

### 3. Funder top-up for an unsubmitted milestone
**Complexity: medium**
There is no way to add funds to a milestone after `create` — if a funder underestimated a
milestone's cost, they must create a whole new escrow. Add a `top_up(id, index, extra)`
that increases a `Pending` milestone's locked amount, funder auth required.
**Acceptance criteria:** increases `milestone.amount` and the contract's token balance by
exactly `extra`; fails if the milestone isn't `Pending`; fails for `extra <= 0`.

### 4. Reject duplicate payee addresses in one milestone
**Complexity: trivial**
`validate_payees` currently allows the same address to appear twice in one milestone's
payee list with two separate shares. That's probably not intended (it would pay the same
address twice from two line items) and should be rejected explicitly, with a test proving
it.
**Acceptance criteria:** `create` with a duplicate payee address in one milestone's payee
list fails with `InvalidInput`; a test covers it.

### 5. Cap `review_window` to a sane maximum
**Complexity: medium**
`review_window` is currently only checked for `> 0` — a funder could set it to
`u64::MAX`, which (combined with the TTL extension math in `storage.rs`) could create
milestones that outlive the contract's own storage TTL assumptions. Pick and document a
reasonable upper bound (e.g. 180 days) and reject anything past it.
**Acceptance criteria:** `create` rejects a `review_window` above the chosen cap; the cap is
documented in the README's known limits or interface section.

### 6. Disallow the arbiter from also being a payee or the funder
**Complexity: medium**
Nothing currently stops the funder from naming themselves, or one of the milestone's own
payees, as the arbiter — which defeats the point of an independent third party ruling a
dispute. Decide whether to block this at `create` time (reject such specs) or just document
it as a caller responsibility, and add a test exercising the chosen behavior either way.
**Acceptance criteria:** the chosen behavior (reject at creation, or explicitly documented
as allowed) has a test proving it.

### 7. Per-milestone getter
**Complexity: trivial**
`get(id)` returns the entire `Escrow`, including every milestone. For an escrow with many
milestones, a caller who only needs one milestone's state pays to deserialize all of them.
Add `get_milestone(id, index) -> Milestone` as a cheaper, targeted read.
**Acceptance criteria:** returns the same data as `get(id).milestones.get(index)`; panics
with `BadIndex` for an out-of-range index, `NotFound` for an unknown escrow.

### 8. List pending/submitted milestone indices for an escrow
**Complexity: medium**
A front-end or indexer currently has to fetch the whole escrow and filter client-side to
find which milestones are actionable. Add a read-only helper that returns the indices of
milestones in a given state, to save that round trip for UIs with many milestones.
**Acceptance criteria:** `milestones_in_state(id, state) -> Vec<u32>` returns the correct
subset for each `MilestoneState`; tests cover an escrow with milestones in several states.

## Testing and verification

### 9. Property tests for payout rounding
**Complexity: medium**
The current tests check rounding dust for two or three fixed payee/share combinations.
Add a property-based test (e.g. with `proptest`) that generates random payee counts
(1..=10), random bps splits that sum to 10_000, and random amounts, and asserts the sum of
individual payouts always equals the input amount exactly.
**Acceptance criteria:** the property test runs at least 256 random cases per execution and
passes; it's wired into `cargo test`.

### 10. Invariant test: locked balance equals the sum of unfinished milestones
**Complexity: medium**
There's no test that directly checks the contract's own token balance always equals the sum
of the `amount` of every milestone still in `Pending`, `Submitted`, or `Disputed` state,
across all escrows the contract holds. Add a helper that computes this invariant and call it
after every state-changing operation in a long, multi-escrow integration test.
**Acceptance criteria:** the invariant helper is reusable across tests; a test exercises
several escrows in different states simultaneously and checks it holds after each step.

### 11. TTL extension tests
**Complexity: medium**
`storage.rs` extends TTL on every read and write, but no test currently asserts the TTL
actually moved. Using `env.storage().persistent().get_ttl(&key)` (via the SDK's ledger
testutils), write a test that advances the ledger sequence close to the threshold and
confirms a read or write extends the entry's remaining TTL.
**Acceptance criteria:** a test demonstrates the TTL after a read/write is at least the
`BUMP_TO` constant used in `storage.rs`, with ledger sequence advanced via testutils.

### 12. Resource-cost check at the size limits
**Complexity: high**
No test exercises `create` at its stated limits: 20 milestones, each with 10 payees (200
payee entries total). Add a test that builds an escrow at exactly these limits and measure
the real CPU/memory budget it consumes, to confirm it doesn't approach Soroban's resource
limits before this gets tested on testnet.
**Acceptance criteria:** a test creates, submits, approves, and reads an escrow at the
20x10 limit; the real resource cost is printed and recorded in the PR description (Soroban
testutils expose a budget/cost tracker — see the SDK's `env.cost_estimate()` or equivalent
for the installed version).

### 13. Event payload tests
**Complexity: trivial**
No current test inspects the actual topics/data of an emitted `Created`, `Submitted`,
`Released`, `Disputed`, `Resolved`, or `Refunded` event — only the resulting state and
balances are checked. Add assertions against `env.events().all()` (or the equivalent
testutils call for the installed SDK) for at least one event of each kind.
**Acceptance criteria:** one test per event type asserts both its topics and its data
fields match what the triggering call should have produced.

## Developer experience and repo hygiene

### 14. CHANGELOG.md
**Complexity: trivial**
There's no changelog. Add a `CHANGELOG.md` following Keep a Changelog conventions, seeded
with an "Unreleased" section listing what's in this repo today.
**Acceptance criteria:** file exists, follows the Keep a Changelog format, accurately lists
current functionality under "Unreleased".

### 15. Shell script wrapper for common `stellar contract invoke` calls
**Complexity: trivial**
Interacting with a deployed escrow by hand means writing out long `stellar contract invoke`
commands with manually-encoded arguments. Add a `scripts/` directory with a thin wrapper
(e.g. `scripts/create-escrow.sh`) that takes plain arguments and shells out to the real CLI,
for maintainers and contributors testing against testnet.
**Acceptance criteria:** at least one working script, with a `--help` usage message,
exercised manually against a local or testnet deployment and the real output pasted into
the PR.

### 16. Contract spec sanity check in CI
**Complexity: trivial**
CI currently builds the wasm but never inspects its exported interface. Add a CI step that
runs `stellar contract info interface` (or the installed CLI's equivalent) against the built
wasm and fails if an expected function is missing, catching an accidental signature
change or a function dropped from the public interface.
**Acceptance criteria:** CI step added; demonstrated to fail when a function is temporarily
renamed, and to pass otherwise.

## Research and design (open-ended, high complexity)

### 17. Multi-token escrows
**Complexity: high**
Today one escrow uses exactly one token for every milestone. Investigate what it would take
to let different milestones in the same escrow use different SEP-41 tokens — storage
layout changes, validation changes, and the payout/refund code paths all need rethinking.
This issue is for the design writeup and a proof-of-concept branch, not a drop-in change to
`main`.
**Acceptance criteria:** a written design doc covering storage layout, migration for
existing escrows (if any exist on a live network), and the validation/payout code changes
needed, plus a proof-of-concept branch demonstrating it compiles and passes a basic test.

### 18. Arbiter committee (N-of-M resolution)
**Complexity: high**
A single arbiter is a single point of trust (and failure) for every dispute. Investigate
replacing the single `arbiter: Address` with an N-of-M committee that must reach quorum
before `resolve` succeeds. This is a significant interface change and needs a design
writeup before any implementation PR is accepted.
**Acceptance criteria:** a written design doc covering the quorum mechanism, how committee
members are chosen and replaced, and the backward-compatibility story for escrows created
under the current single-arbiter model.

### 19. Non-standard token behavior audit
**Complexity: high**
This contract assumes a well-behaved SEP-41 token (transfers either succeed for the full
amount or fail atomically). Research what happens if someone creates an escrow with a
non-standard or malicious token (one with transfer fees, blocklists, or non-atomic
behavior) and write up what, if anything, needs to change, or what should be documented as
an explicit caller responsibility.
**Acceptance criteria:** a written report covering at least three concrete non-standard
token behaviors and their effect on this contract's invariants, with a recommendation
(code change vs. documentation) for each.

### 20. Formal model of the milestone state machine
**Complexity: high**
Write a formal (or at least rigorously structured) model of the milestone state machine —
every state, every transition, every precondition — independent of the Rust
implementation, and use it to check the real code doesn't have an unreachable state or a
missing transition guard. TLA+, a hand-written transition table, or a property-test model
(e.g. driving `cargo test` with randomized transition sequences) are all acceptable
approaches; the issue is open on which.
**Acceptance criteria:** a written model exists in `docs/`; at least one real code path or
test gap is found and reported (filed as a separate issue) as a result of writing it, or
the writeup explicitly states none was found and why confidence is high.
