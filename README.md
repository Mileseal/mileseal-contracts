# mileseal-contracts

A milestone escrow smart contract for Stellar Soroban. Part of
[Mileseal](https://github.com/Johnero542), built for the
[Drips Stellar Wave Program](https://drips.network/wave).

**Status: contract implemented and unit-tested locally. Not yet audited. Not yet deployed
to any network.** See [Known limits](#known-limits) and [Testnet deployment](#testnet-deployment)
below.

## What it is

A funder locks the full payment for every milestone of a project up front, in a single
Stellar Asset Contract (SAC) token. Each milestone has its own deadline and one or more
payees, each owed a fixed percentage share. From there:

- A payee submits a hash of their deliverable. If the funder approves, every payee on that
  milestone is paid their share.
- If the funder goes silent, any payee can claim payment themselves once a review window
  has passed.
- If the funder disputes within the review window, an arbiter chosen at creation time rules
  what percentage goes to the payees (split by their own shares) and what returns to the
  funder.
- If nobody submits before the deadline, the milestone's funds return to the funder.

## State machine

Each milestone moves through these states independently of the others in the same escrow:

| State       | Reached by                                          | Can move to                  |
|-------------|------------------------------------------------------|-------------------------------|
| `Pending`   | `create` (initial state)                             | `Submitted`, `Refunded`       |
| `Submitted` | `submit` (before the deadline)                       | `Released`, `Disputed`        |
| `Released`  | `approve`, or `claim_unreviewed` after the funder goes silent | *(terminal)*            |
| `Disputed`  | `dispute` (funder, within the review window)         | `Resolved`                    |
| `Resolved`  | `resolve` (arbiter rules a split)                    | *(terminal)*                  |
| `Refunded`  | `refund_expired` (deadline passed, never submitted)  | *(terminal)*                  |

## Contract interface

Source of truth: [`contracts/escrow/src/lib.rs`](contracts/escrow/src/lib.rs). Amounts are
`i128` token stroops; timestamps are unix seconds (`u64`); shares are basis points
(`10_000` = 100%).

| Function | Auth | Effect |
|---|---|---|
| `create(funder, arbiter, token, review_window, specs) -> u64` | funder | Validates every milestone spec, locks the sum of all amounts from `funder`, returns the new escrow id. |
| `submit(id, index, payee, proof)` | payee | Moves a `Pending` milestone (before its deadline) to `Submitted`. |
| `approve(id, index)` | funder | Pays every payee their share of a `Submitted` milestone; moves it to `Released`. |
| `claim_unreviewed(id, index, payee)` | payee | Same payout as `approve`, callable by any payee once the review window has elapsed with no funder action. |
| `dispute(id, index)` | funder | Moves a `Submitted` milestone to `Disputed`, before the review window elapses. |
| `resolve(id, index, payee_bps)` | arbiter | Splits a `Disputed` milestone's amount: `payee_bps` (0..=10_000) to the payees by their own shares, the rest back to the funder. Moves it to `Resolved`. |
| `refund_expired(id, index)` | anyone | Returns a `Pending` milestone's funds to the funder once its deadline has passed with no submission. |
| `get(id) -> Escrow` | — | Reads the full state of an escrow. |
| `count() -> u64` | — | How many escrows have been created. |

Payout rounding: every payee but the last on a milestone gets `floor(amount * bps /
10_000)`; the last payee gets whatever remains, so a milestone's payouts always sum to
exactly its locked amount.

## Running the checks locally

```sh
# Format
cargo fmt --check

# Lint (warnings are errors)
cargo clippy --all-targets -- -D warnings

# Unit tests (21 tests covering the full lifecycle; run with cargo test --
# --ignored to also run the fixture-dump test used by mileseal-sdk)
cargo test
```

## Building the wasm

Requires the [stellar CLI](https://developers.stellar.org/docs/tools/cli) and the
`wasm32v1-none` Rust target:

```sh
rustup target add wasm32v1-none
stellar contract build
```

The optimized wasm is written to `target/wasm32v1-none/release/mileseal_escrow.wasm`.

## Testnet deployment

Not yet deployed. Once a maintainer has a funded testnet identity, the contract is
deployed with the real `stellar contract deploy` command for the installed CLI version
(see `stellar contract deploy --help`), and the resulting id recorded here:

**Testnet Contract ID:** _(not yet deployed)_

## Known limits

- A disputed milestone stays locked forever if the arbiter never rules; there is no
  dispute timeout.
- Only the funder can dispute a submission; payees have no equivalent recourse against a
  bad-faith funder beyond the review-window claim.
- Milestones cannot be amended or cancelled once an escrow is created.
- One token per escrow; it cannot mix assets across milestones.
- This contract has not been audited.

## Contributing via Drips Wave

This repo is built for submission to the [Drips Stellar Wave Program](https://drips.network/wave).
Scoped issues are labeled `complexity: trivial` (100 points), `complexity: medium` (150
points), or `complexity: high` (200 points). See [CONTRIBUTING.md](CONTRIBUTING.md) for how
to pick up an issue and the checks your pull request needs to pass.
