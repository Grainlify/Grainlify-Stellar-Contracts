# Test-Port Feasibility Audit: Issue #245 (`soroban/contracts/program-escrow`)

**Date:** 2026-07-23
**Scope:** Documentation-only audit. No contract logic or test code was changed.
**References:** Issue #245 ("Port missing edge-case tests from program-escrow into soroban/contracts/program-escrow/test.rs"). Follow-up to [PROGRAM_ESCROW_DUPLICATION_AUDIT.md](PROGRAM_ESCROW_DUPLICATION_AUDIT.md).

## Verdict

Issue #245's Step 1 ("Diff the public function surface of `soroban/contracts/program-escrow/src/lib.rs` against `program-escrow/src/lib.rs` to find under-tested overlapping functions") returns **zero overlap**, because the `soroban` side is **still** the unmodified hello-world placeholder produced by `stellar contract init` (the same conclusion as the prior duplication audit in this folder; nothing has changed in `soroban/contracts/program-escrow/` since that audit).

Every requested coverage area — reentrancy guard, milestone / balance invariants, RBAC negative paths, pause / whitelist interaction — exists on the `program-escrow` source side only. The `soroban` copy has neither the state, the storage, the auth gates, nor the token transfers needed to express those tests in any security-meaningful way. Producing nominal test rows (e.g. a `test_reentrancy` that calls `hello()` twice) would not exercise the risk surface the issue names and would *misrepresent* coverage to future audits.

The issue itself prescribes this exact outcome in §"Suggested execution": *"Flag any behavioral divergence discovered during porting as a follow-up, rather than silently reconciling it in this issue."* This document is that flag.

## What Step 1 of the issue would compute

| Aspect | `program-escrow/src/lib.rs` | `soroban/contracts/program-escrow/src/lib.rs` |
|---|---|---|
| Lines of code | 3,623 | 23 |
| Public functions (selected) | `init_program`, `initialize_program`, `batch_initialize_programs`, `lock_program_funds`, `batch_payout`, `single_payout`, `set_paused`, `get_pause_flags`, `set_circuitadmin`, `reset_circuit_breaker`, `emergency_open_circuit`, `configure_circuit_breaker`, `open_dispute` / `resolve_dispute` / `cancel_dispute` (and recipient / schedule-scoped variants), `set_whitelist`, `set_whitelist_enforced`, `is_whitelisted`, `set_fund_cap_config`, `get_fund_cap_config`, governance integration (`set_governance_contract`, `set_min_governance_version`), monitoring surface, release-schedule surface (`create_program_release_schedule`, `trigger_program_releases`, etc.), history-query / aggregate-stats surface, circuit-breaker status surface, etc. | `hello(env, to)` (single public function) |
| State | Persistent + instance storage, explicit TTL extension via `bump_persistent_symbol_ttl`, `bump_persistent_datakey_ttl`, `bump_instance_ttl`; multi-program registry via `DataKey::Program`. | None |
| Auth model | `require_auth()` on admin / authorized payout key; pause flags; whitelist enforcement; dispute gates. | None |
| Token transfers | Yes (SAC-compatible `token::Client::transfer` / `transfer_from`). | None |
| Companion test surface | 20+ specialized test modules under `program-escrow/src/`: reentrancy (`reentrancy_tests`, `malicious_reentrant`, `reentrancy_guard_standalone_test`), RBAC (`rbac_tests`), balance invariants (`test_balance_invariant`, plus the property-fuzz and stress tests already in `test.rs`), pause / circuit breaker (`test_granular_pause`, `test_circuit_breaker_integration`, `test_pause`), whitelist / anti-abuse (`test_whitelist`, `test_anti_abuse_whitelist_bypass`), dispute resolution (`test_dispute_resolution`), governance (`test_governance_integration`, `governance_integration`), monitoring (`test_monitoring`, `monitoring`), analytics events (`test_analytics_events`), error recovery (`error_recovery`, `error_recovery_tests`), lifecycle / budget profiling (`test_lifecycle`, `budget_profiling_tests`), the dedicated `test_issue_189`, plus the top-level `test.rs` of integration + gas-proxy + batch + analytics + history-query tests. | `test.rs` containing exactly one test (`test()`) that calls `client.hello()`. |

**Intersection of the two public function surfaces: ∅.** Step 1 of the issue has an empty input set — there are no "under-tested overlapping functions."

## Why each requested coverage area cannot be ported

| Issue's target | Required surface on the target | Available on the `soroban` placeholder? |
|---|---|---|
| Reentrancy guard | A function that performs state-changing, observable side effects (token transfers / state writes) where a reentrant call could corrupt an invariant, plus a writable reentrancy flag like `reentrancy_guard`. | **No.** `hello()` reads no state and writes none. A test that re-enters `hello()` cannot trip a guard. |
| Milestone / balance invariants | `lock_program_funds`, `batch_payout`, `single_payout`, `get_remaining_balance`, `get_program_info`, and a `ProgramData { total_funds, remaining_balance, payout_history }` field to compare against on-chain token balance. | **No.** No `ProgramData`, no token client, no balance field. |
| RBAC negative paths | `setadmin`, `initialize_contract`, `set_paused`, `set_circuitadmin`, `set_whitelist`, etc., each with `require_auth()` semantics whose failure we're trying to assert. | **No.** No `require_auth()`, no admin gate, no `set_paused`. |
| Pause / whitelist interaction | `set_paused`, `get_pause_flags`, `set_whitelist`, `set_whitelist_enforced`, and a guarded call site (e.g. `batch_payout`'s pause + whitelist block) that consumes them. | **No.** No pause flags, no whitelist map, no consuming call site. |

What *can* concretely be done against `hello()`:

- Boundary checks on input (empty `to`, very long `to`, unicode).
- Repeat-call equality (does the contract return the same vector twice?).
- Idempotence / determinism across `env.ledger().with_frame()` cycles.

These tests have no security meaning for the program-escrow surface and satisfy none of the issue's four acceptance areas.

## Existing fix parallels

The prior duplication audit in this folder already concluded that:

- The `soroban/contracts/program-escrow` placeholder is **not** a stale duplicate of the real contract — it is the `stellar contract init` output from the workspace scaffold step.
- It exists solely as a toolchain / SDK-matrix smoke-test target for `scripts/run_contract_matrix.sh` and CI's `soroban-contract-matrix` job.
- The same logic applies to the sibling `soroban/contracts/escrow/src/lib.rs`.

This audit confirms that conclusion has not changed: as of this date, `soroban/contracts/program-escrow/src/lib.rs` is the same 23-line hello-world file as when the prior audit was written.

## Recommended path (out of scope for this feasibility audit)

Per the same logic as the prior duplication audit, the clean fix is to mirror the production `program-escrow` crate into `soroban/contracts/program-escrow` first, **then** port tests. Concretely:

1. **Decide crate topology.** Either (a) keep `soroban/contracts/program-escrow` as a separate crate and mirror the real contract into it, or (b) point CI's `soroban-contract-matrix` job at the existing `program-escrow` crate and delete the placeholder (the prior audit already proposed both options).
2. **If mirroring, reconcile SDK + deps.** The `soroban/` workspace pins `soroban-sdk = "=23"` while `program-escrow/Cargo.toml` pins `soroban-sdk = "21.0.0"` and depends on `grainlify-core = { path = "../grainlify-core" }`. A wholesale copy must update the SDK pin, add `grainlify-core` or its equivalent, and re-validate compilation in the `soroban/` workspace.
3. **Copy module files.** `program-escrow/src/lib.rs` references `mod error_recovery`, `mod governance_integration`, `pub mod monitoring`, `mod reentrancy_guard`; these must travel with it.
4. **Only then port the four target test areas** into `soroban/contracts/program-escrow/src/test.rs` (or new sibling test modules mirroring `program-escrow/src/`'s split).

Until steps 1–4 happen, issue #245's acceptance criteria cannot legitimately be met with new tests against the current target — and **shouldn't** be, because the result would be tests that don't test anything.

## This audit does not perform any of steps 1–4

The user explicitly chose *"Just file the divergence flag"* as the action for this turn. Therefore:

- `soroban/contracts/program-escrow/src/lib.rs` was **not** modified.
- `soroban/contracts/program-escrow/src/test.rs` was **not** modified.
- No fake / placeholder tests were added.

The deliverable is this document. Resolving the underlying gap (real contract + real tests in the `soroban/` crate) is left as a follow-up issue, exactly as the user request prescribes.

## Suggested follow-up issue tracker entries

1. **Mirror `program-escrow` crate into `soroban/contracts/program-escrow`** — copy `lib.rs` plus module files, update Cargo deps, validate workspace compile. *Prerequisite for #245.*
2. **Re-issue test-port after #1 merges** — port reentrancy / RBAC / balance-invariant / pause-whitelist coverage from `program-escrow/src/*.rs` modules into `soroban/contracts/program-escrow/src/` test modules.
3. **Apply the same fix-package to `soroban/contracts/escrow`** — the prior duplication audit flagged both placeholders; this audit confirms the same template-still-present state.
