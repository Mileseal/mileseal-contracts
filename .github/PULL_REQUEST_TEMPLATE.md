## Closes

<!-- Closes #<issue-number>. Every PR must reference the issue it was assigned for. -->

## What changed

<!-- A short description of the change. -->

## How I verified it

<!-- Real command output, not a claim. For example:
- `cargo test` output showing new/changed tests passing
- `cargo clippy --all-targets -- -D warnings` output
- `stellar contract build` output if wasm-affecting
-->

## Checklist

- [ ] `cargo fmt --check` passes
- [ ] `cargo clippy --all-targets -- -D warnings` passes
- [ ] `cargo test` passes, including new tests for this change
- [ ] No stub functions, placeholder logic, or `TODO` comments
- [ ] Doc comments added/updated for any public item touched
- [ ] If a privileged function was touched, a wrong-signer test exists for it
