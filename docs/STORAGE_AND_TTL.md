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
| `DataKey::Escrow(u64)` | Persistent | Per-bounty escrow record and funds state. | No explicit extension in the reviewed lock/release/refund paths; archival can block fund recovery until restored. |
| `DataKey::EscrowIndex` | Persistent | Escrow listing/index state. | No explicit extension observed. |
| `DataKey::DepositorIndex(Address)` | Persistent | Depositor-to-escrow lookup state. | No explicit extension observed. |
| `DataKey::FeeConfig` | Instance/persistent by call site | Fee policy used by escrow operations. | Treat as configuration that should remain live with the instance; verify call site before changing. |
| `DataKey::RefundApproval(u64)` | Persistent | Multisig or admin approval state for refunds. | No explicit extension observed; should live at least through the claim/refund window. |
| `DataKey::ReentrancyGuard` | Instance | Short-lived in-call guard. | Removed after protected calls; no extension needed. |
| `DataKey::MultisigConfig` | Instance | Multisig configuration. | No explicit per-key extension; covered by contract instance TTL. |
| `DataKey::ReleaseApproval(u64)` | Persistent | Approval state for releases. | No explicit extension observed; should live at least through release execution. |
| `DataKey::PendingClaim(u64)` | Persistent | Pending claim state by bounty id. | No explicit extension observed; should align with `ClaimWindow`. |
| `DataKey::ClaimWindow` | Instance | Claim-window configuration. | No explicit per-key extension; covered by contract instance TTL. |
| `DataKey::PauseFlags` | Instance | Granular pause flags. | No explicit per-key extension; covered by contract instance TTL. |
| `DataKey::AmountPolicy` | Instance | Amount/range policy configuration. | No explicit per-key extension; covered by contract instance TTL. |
| `AntiAbuseKey::Admin` | Instance | Anti-abuse module admin. | No explicit per-key extension; covered by contract instance TTL. |
| `AntiAbuseKey::Config` | Instance | Anti-abuse module configuration. | No explicit per-key extension; covered by contract instance TTL. |
| Anti-abuse state key | Persistent | Per-actor abuse/rate state. | Explicitly extends TTL with `extend_ttl(&key, 17280, 17280)`. |
| Monitoring symbols such as `op_count`, `usr_count`, `err_count`, `perf_cnt`, `perf_time` | Persistent | Operational counters and metrics. | No explicit extension observed; loss affects monitoring continuity rather than escrow ownership. |

## program-escrow

Source reviewed: `program-escrow/src/lib.rs`.

| Key | Storage class | Purpose | Current TTL behavior |
| --- | --- | --- | --- |
| `PROGRAM_DATA` / `DataKey::Program(String)` | Instance | Program configuration and mutable program state. | No explicit per-key extension; covered by contract instance TTL. |
| `DataKey::Admin` | Instance | Program escrow administrator. | No explicit per-key extension; covered by contract instance TTL. |
| `SCHEDULES` / `DataKey::ReleaseSchedule(String, u64)` | Instance | Release schedule entries. | No explicit extension observed; schedule lifetime should cover all release deadlines plus dispute/recovery buffer. |
| `RELEASE_HISTORY` / `DataKey::ReleaseHistory(String)` | Instance | Historical release records. | No explicit extension observed; preserve for auditability. |
| `NEXT_SCHEDULE_ID` / `DataKey::NextScheduleId(String)` | Instance | Next release schedule id. | No explicit per-key extension; covered by contract instance TTL. |
| `DataKey::PayoutApproval(String, Address)` | Instance | Payout approval state. | No explicit extension observed; should remain live through payout execution. |
| `DataKey::PendingClaim(String, u64)` | Instance | Pending claim by program and schedule id. | No explicit extension observed; align with claim window. |
| `DataKey::ClaimWindow` | Instance | Claim-window configuration. | No explicit extension observed. |
| `DataKey::PauseFlags` | Instance | Pause controls. | No explicit extension observed. |
| `DataKey::RateLimitConfig` | Instance | Rate-limit configuration. | No explicit extension observed. |
| `DataKey::FeeConfig` | Instance | Fee configuration. | No explicit extension observed. |
| `PROGRAM_REGISTRY` / `DataKey::ProgramRegistry` | Instance | Registry of known programs. | No explicit extension observed. |
| `DataKey::Dispute` | Instance | Current dispute record. | No explicit extension observed; should live through dispute resolution and appeal buffer. |
| Program registration mirror written during creation | Persistent | Program metadata/index written alongside instance program creation. | No explicit extension observed; future TTL work should decide whether this index must outlive instance program data. |

## grainlify-core

Source reviewed: `grainlify-core/src/lib.rs`.

| Key | Storage class | Purpose | Current TTL behavior |
| --- | --- | --- | --- |
| `DataKey::Admin` | Instance | Core administrator. | No explicit per-key extension; covered by contract instance TTL. |
| `DataKey::Version` | Instance | Current core contract version. | No explicit per-key extension; covered by contract instance TTL. |
| `DataKey::MigrationState` | Instance | Active migration state. | No explicit per-key extension; must remain live through migration finalization. |
| `DataKey::PreviousVersion` | Instance | Previous version marker. | No explicit per-key extension; preserve for rollback/audit while migration remains relevant. |
| Monitoring symbols such as `op_count`, `usr_count`, `err_count`, `perf_cnt`, `perf_time` | Persistent | Operational counters and metrics. | No explicit extension observed. |

## TTL sizing guidance

Use deadlines and recovery windows rather than a single project-wide constant:

- Escrow funds state should remain live for the maximum of bounty/program deadline, claim window, dispute window, refund window, and an operational recovery buffer.
- Approval and pending-claim entries should remain live until the underlying action can no longer be executed, then may be removed or allowed to expire after audit requirements are satisfied.
- Index and registry entries should be extended whenever the referenced live escrow/program entry is extended; otherwise list queries can diverge from recoverable funds state.
- Configuration entries in instance storage should be protected by contract-instance TTL extension in deployment, upgrade, and maintenance runbooks.
- Monitoring counters can use shorter retention if dashboards tolerate gaps, but that decision should be documented separately from funds-safety state.

## Related follow-up work

- #75 adds explicit persistent storage TTL extension for `bounty_escrow` fund paths.
- #76 adds persistent storage TTL extension for program escrow plans, release history, and program data.

## Validation notes

The key inventory was cross-checked against the current `DataKey` enums and storage call sites in:

```
grep -n "DataKey" bounty_escrow/contracts/escrow/src/lib.rs program-escrow/src/lib.rs grainlify-core/src/lib.rs
```

The review also checked `storage().instance()`, `storage().persistent()`, and `extend_ttl` call sites to distinguish singleton instance state from persistent escrow, index, monitoring, and anti-abuse state.
