# Contributing to mileseal-contracts

This repo is part of [Mileseal](https://github.com/Johnero542), a milestone escrow on
Stellar Soroban, built for the [Drips Stellar Wave Program](https://drips.network/wave).

## Before you start

**Do not start work on an issue until a maintainer has assigned it to you.** Apply to an
issue via its Drips Wave listing (or by commenting, if the issue says so) and wait for the
assignment. Unassigned pull requests may be closed without review.

## Branches and pull requests

- Branch names: `feat/<issue-number>-short-name` or `fix/<issue-number>-short-name`
  (for example `feat/12-dispute-timeout`).
- Your pull request description must reference the issue it closes (for example
  `Closes #12`).
- Keep pull requests scoped to one issue. Small, focused diffs review faster.

## Running the checks locally

This repo is a Cargo workspace with one crate, `contracts/escrow`.

```sh
# Format
cargo fmt --check

# Lint (warnings are errors)
cargo clippy --all-targets -- -D warnings

# Unit tests
cargo test

# Build the contract to wasm (requires the stellar CLI and the
# wasm32v1-none Rust target: rustup target add wasm32v1-none)
stellar contract build
```

All four must pass before a pull request is reviewed. CI runs the same checks.

## Code style

- No stub functions, placeholder logic, or `TODO` comments in a pull request meant to be
  merged. If a piece of work is partial, say so in the PR description and scope the issue
  down instead.
- Validate every input; use checked arithmetic (`checked_add`, `checked_mul`, ...), never
  raw `+`/`*`/`-` on amounts or timestamps.
- Every public contract function and type needs a doc comment explaining what it does, not
  just restating its name.
- If you touch a privileged function (one that calls `require_auth` on a specific party),
  add or update a test that calls it with the **wrong** signer via `mock_auths` and expects
  failure. `mock_all_auths` alone does not prove an auth check exists.

## Reporting issues

Please use the issue templates (bug/feature are not applicable here; all work arrives as
scoped `complexity: trivial` / `complexity: medium` / `complexity: high` issues from the
maintainer). Do not open freeform issues; the issue form will guide you.
