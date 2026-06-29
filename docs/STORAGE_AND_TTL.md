# Storage and TTL model

This document maps the Soroban storage classes used by the Grainlify contracts and records the current TTL and archival assumptions that future storage-extension work should preserve.

## Soroban storage classes

Soroban instance storage is scoped to the contract instance and is appropriate for compact configuration or singleton state that should move with the contract instance. Persistent storage is keyed contract data intended to survive independently and can be archived if TTL is not extended. Temporary storage is not currently used by the reviewed contracts.

Unextended persistent entries that represent escrow balances, claims, approvals, indexes, or analytics can become a funds-safety or operability risk because archived entries must be restored before they can be read or mutated. Instance entries also rely on the contract instance lifetime, so deployment and upgrade runbooks should include contract-instance TTL extension as part of maintenance.

## bounty_escrow

Source reviewed: `bounty_escrow/contracts/escrow/src/lib.rs`.

| Key | Storage class | Purpose | Current TTL behavior |
| --- | --- | --- | --- |
| `DataKey::Admin` | Instance | Contract administrator and authorization root. | No explicit per-key extension; covered by contract instance TTL. |
| `DataKey::Token` | Instance | Token contract address used for escrow transfers. | No explicit per-key extension; covered by contract instance TTL. |
| `DataKey::Escrow(u64)` | Persistent | Per-bounty escrow record and funds state. | Extended by `bump_escrow_ttl()` using `extend_ttl(..., ESCROW_TTL_THRESHOLD, ESCROW_TTL_EXTEND_TO)`. Without extension, archived escrow state can block fund recovery. |
| `DataKey::EscrowIndex` | Persistent | Escrow listing/index state. | Extended by `bump_escrow_index_ttl()` with the same escrow TTL policy. |
| `DataKey::DepositorIndex(Address)` | Persistent | Depositor-to-escrow lookup state. | Extended by `bump_escrow_index_ttl()` when a depositor index is updated. |
| `DataKey::AggregateCounters` | Persistent | O(1) aggregate escrow metrics for summary queries. | Extended on update via `extend_ttl(..., ESCROW_TTL_THRESHOLD, ESCROW_TTL_EXTEND_TO)`. |
| `DataKey::FeeConfig` | Instance | Fee policy used by escrow operations. | Stored in instance storage; no explicit per-key TTL extension beyond contract instance TTL. |
| `DataKey::RefundApproval(u64)` | Persistent | Multisig/admin approval state for refunds. | No explicit extension observed; should remain alive through refund window and dispute resolution. |
| `DataKey::ReentrancyGuard` | Instance | Short-lived in-call guard. | Removed after protected calls; no extension needed. |
| `DataKey::MultisigConfig` | Instance | Multisig configuration. | No explicit per-key extension; covered by contract instance TTL. |
| `DataKey::ReleaseApproval(u64)` | Persistent | Approval state for releases. | No explicit extension observed; should remain alive through release execution. |
| `DataKey::PendingClaim(u64)` | Persistent | Pending claim state by bounty id. | No explicit extension observed; should align with `ClaimWindow` and dispute-resolution windows. |
| `DataKey::ClaimWindow` | Instance | Claim-window configuration. | No explicit per-key extension; covered by contract instance TTL. |
| `DataKey::PauseFlags` | Instance | Granular pause flags. | No explicit per-key extension; covered by contract instance TTL. |
| `DataKey::AmountPolicy` | Instance | Amount/range policy configuration. | No explicit per-key extension; covered by contract instance TTL. |
| `AntiAbuseKey::Admin` | Instance | Anti-abuse module admin. | No explicit per-key extension; covered by contract instance TTL. |
| `AntiAbuseKey::Config` | Instance | Anti-abuse module configuration. | No explicit per-key extension; covered by contract instance TTL. |
| `AntiAbuseKey::Whitelist(Address)` | Instance | Per-address whitelist for rate-limit bypass. | No explicit per-key extension; covered by contract instance TTL. |
| `AntiAbuseKey::State(Address)` | Persistent | Per-address anti-abuse state. | Explicitly extends TTL with `extend_ttl(&key, 17280, 17280)` after each rate-limit update. |
| Monitoring symbols such as `op_count`, `usr_count`, `err_count`, `perf_cnt`, `perf_time`, `perf_last` | Persistent | Operational counters and metrics. | No explicit extension observed; loss affects monitoring continuity but not escrow balance safety. |

## program-escrow

Source reviewed: `program-escrow/src/lib.rs`.

> Note: `program-escrow` uses `storage().instance()` for all reviewed state and does not use `storage().persistent()` in the current `src/lib.rs`.

| Key | Storage class | Purpose | Current TTL behavior |
| --- | --- | --- | --- |
| `PROGRAM_DATA` / `DataKey::Program(String)` | Instance | Program configuration and mutable program state. | No explicit per-key extension; covered by contract instance TTL. |
| `DataKey::Admin` | Instance | Program escrow administrator. | No explicit per-key extension; covered by contract instance TTL. |
| `NEXT_SCHEDULE_ID` / `DataKey::NextScheduleId(String)` | Instance | Next release schedule id for a program. | No explicit per-key extension; covered by contract instance TTL. |
| `SCHEDULES` / `DataKey::ReleaseSchedule(String, u64)` | Instance | Release schedule entries. | No explicit extension observed; schedule lifetime should cover all release deadlines plus dispute/recovery buffer. |
| `RELEASE_HISTORY` / `DataKey::ReleaseHistory(String)` | Instance | Historical release records. | No explicit extension observed; preserve for auditability. |
| `DataKey::PayoutApproval(String, Address)` | Instance | Payout approval state. | No explicit extension observed; should remain live through payout execution. |
| `DataKey::PendingClaim(String, u64)` | Instance | Pending claim by program and schedule id. | No explicit extension observed; align with claim window and dispute resolution. |
| `DataKey::ClaimWindow` | Instance | Claim-window configuration. | No explicit extension observed; covered by contract instance TTL. |
| `DataKey::PauseFlags` | Instance | Pause controls. | No explicit extension observed; covered by contract instance TTL. |
| `DataKey::RateLimitConfig` | Instance | Rate-limit configuration. | No explicit extension observed; covered by contract instance TTL. |
| `DataKey::FeeConfig` | Instance | Fee configuration. | No explicit extension observed; covered by contract instance TTL. |
| `PROGRAM_REGISTRY` / `DataKey::ProgramRegistry` | Instance | Registry of known programs. | No explicit extension observed; should remain consistent with live programs. |
| `DataKey::Dispute` | Instance | Current dispute record. | No explicit extension observed; should live through dispute resolution and settlement. |

## grainlify-core

Source reviewed: `grainlify-core/src/lib.rs`.

| Key | Storage class | Purpose | Current TTL behavior |
| --- | --- | --- | --- |
| `DataKey::Admin` | Instance | Core administrator. | No explicit per-key extension; covered by contract instance TTL. |
| `DataKey::Version` | Instance | Current core contract version. | No explicit per-key extension; covered by contract instance TTL. |
| `DataKey::MigrationState` | Instance | Active migration state. | No explicit per-key extension; must remain live through migration finalization. |
| `DataKey::PreviousVersion` | Instance | Previous version marker. | No explicit per-key extension; preserve for rollback/audit while migration remains relevant. |
| Monitoring symbols such as `op_count`, `usr_count`, `err_count`, `perf_cnt`, `perf_time`, `perf_last` | Persistent | Operational counters and metrics. | No explicit extension observed. |

## Security note

Unextended persistent entries that hold escrow, approval, claim, index, or audit state create an archival risk. If these entries are archived, the contract can still exist, but fund-recovery and dispute-resolution operations may be blocked until the archived state is restored.

## TTL sizing guidance

Use deadline and recovery horizons rather than a single project-wide constant:

- Escrow and program fund state should remain live for the maximum of bounty/program deadline, claim window, dispute window, refund window, and an operational recovery buffer.
- Approval and pending-claim entries should remain live until the underlying action can no longer be executed, then may be removed or allowed to expire only after audit requirements are satisfied.
- Index and registry entries should be extended whenever the referenced live escrow/program entry is extended; otherwise list queries can diverge from recoverable state.
- Configuration entries in instance storage should be protected by contract-instance TTL extension in deployment, upgrade, and maintenance runbooks.
- Monitoring counters can use shorter retention if dashboards tolerate gaps, but that decision must be documented separately from funds-safety state.
- For 5-second ledgers, the reviewed `bounty_escrow` anti-abuse TTL extension of `17280` approximates one day, while escrow fund state should use a longer horizon such as the 30-day equivalent signaled by `ESCROW_TTL_EXTEND_TO`.

## Related follow-up work

- #75 adds explicit persistent storage TTL extension for `bounty_escrow` fund paths.
- #76 adds persistent storage TTL extension for `program-escrow` plans, release history, and program data.
- #99 requests full storage-key documentation across `bounty_escrow`, `program-escrow`, and `grainlify-core`.

## Validation notes

The key inventory and storage model were cross-checked against the current `DataKey` enums and storage call sites with:

```bash
grep -n "DataKey" bounty_escrow/contracts/escrow/src/lib.rs program-escrow/src/lib.rs grainlify-core/src/lib.rs

grep -R "storage().instance()\|storage().persistent()\|extend_ttl" bounty_escrow/contracts/escrow/src/lib.rs program-escrow/src/lib.rs grainlify-core/src/lib.rs
```

The review also confirmed that `program-escrow` currently has no `storage().persistent()` accesses in `src/lib.rs`.
