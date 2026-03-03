# Event Schema

This repository currently supports two event payload generations:

- `v1` (legacy): unversioned payloads (required fields only)
- `v2` (current): payload map containing a `version: 2` field plus required fields

## Required Compatibility Fields

Indexers and SDK consumers must be able to parse these required fields across versions:

- `version` (optional in v1, required in v2+)
- `amount` (when event type includes value transfer semantics)

Additional fields are considered additive and should be ignored by forward-compatible parsers.


## Table of Contents

- [Event Schema](#event-schema)
  - [Required Compatibility Fields](#required-compatibility-fields)
  - [Table of Contents](#table-of-contents)
  - [1. Overview](#1-overview)
  - [2. Schema Versioning Policy](#2-schema-versioning-policy)
  - [3. How Events Are Emitted (Soroban Mechanics)](#3-how-events-are-emitted-soroban-mechanics)
  - [4. How Events Are Consumed](#4-how-events-are-consumed)
    - [Via Stellar RPC (TypeScript/JS)](#via-stellar-rpc-typescriptjs)
    - [Via Trustless Work API](#via-trustless-work-api)
    - [Via SubQuery Indexer](#via-subquery-indexer)
  - [5. Contract: `bounty_escrow`](#5-contract-bounty_escrow)
    - [5.1 `BountyEscrowInitialized`](#51-bountyescrowinitialized)
      - [v2 Payload Example](#v2-payload-example)
    - [5.2 `FundsLocked`](#52-fundslocked)
      - [v2 Payload Example](#v2-payload-example-1)
    - [5.3 `FundsReleased`](#53-fundsreleased)
      - [v2 Payload Example](#v2-payload-example-2)
    - [5.4 `FundsRefunded`](#54-fundsrefunded)
      - [v2 Payload Example](#v2-payload-example-3)
    - [5.5 `FeeCollected`](#55-feecollected)
      - [Payload Example](#payload-example)
    - [5.6 `BatchFundsLocked`](#56-batchfundslocked)
      - [Payload Example](#payload-example-1)
    - [5.7 `BatchFundsReleased`](#57-batchfundsreleased)
      - [Payload Example](#payload-example-2)
    - [5.8 `ApprovalAdded`](#58-approvaladded)
      - [Payload Example](#payload-example-3)
    - [5.9 `FeeConfigUpdated`](#59-feeconfigupdated)
      - [Payload Example](#payload-example-4)
    - [5.10 `PauseStateChanged` (bounty)](#510-pausestatechanged-bounty)
      - [Payload Example](#payload-example-5)
  - [6. Contract: `program_escrow`](#6-contract-program_escrow)
    - [6.1 `ProgramInitialized`](#61-programinitialized)
      - [v2 Payload Example](#v2-payload-example-4)
    - [6.2 `FundsLocked` (program)](#62-fundslocked-program)
      - [v2 Payload Example](#v2-payload-example-5)
    - [6.3 `BatchPayout`](#63-batchpayout)
      - [v2 Payload Example](#v2-payload-example-6)
    - [6.4 `Payout`](#64-payout)
      - [v2 Payload Example](#v2-payload-example-7)
    - [6.5 `PauseStateChanged` (program)](#65-pausestatechanged-program)
      - [Payload (raw tuple)](#payload-raw-tuple)
  - [7. Contract: `grainlify-core`](#7-contract-grainlify-core)
    - [7.1 `MigrationEvent`](#71-migrationevent)
      - [Payload Example (success)](#payload-example-success)
      - [Payload Example (failure — emitted before panic)](#payload-example-failure--emitted-before-panic)
    - [7.2 `OperationMetric`](#72-operationmetric)
      - [Payload Example](#payload-example-6)
    - [7.3 `PerformanceMetric`](#73-performancemetric)
      - [Payload Example](#payload-example-7)
  - [8. Event Topic Reference](#8-event-topic-reference)
  - [9. Payload Field Reference](#9-payload-field-reference)
  - [10. v1 → v2 Migration Guide](#10-v1--v2-migration-guide)
  - [11. Forward-Compatible Parsing](#11-forward-compatible-parsing)
  - [12. Security Notes](#12-security-notes)
  - [13. Test Coverage Notes](#13-test-coverage-notes)
  - [14. Inline Source References](#14-inline-source-references)
  - [15. Changelog](#15-changelog)

---

## 1. Overview

This document is the authoritative, versioned event schema for all Soroban smart contracts
in this repository, generated from a direct audit of the source code. It supersedes any
previously hand-authored schema.

All meaningful lifecycle transitions emit on-chain **Soroban contract events** consumable by:

- The Trustless Work API (real-time notifications, webhooks)
- Third-party indexers (SubQuery, Horizon event streams)
- SDK consumers subscribing to contract state changes without polling

Two payload generations are supported:

| Generation | `version` field | Status |
|------------|-----------------|--------|
| `v1`       | Absent          | Legacy — backward-compatible, required fields only |
| `v2`       | `2` (`u32`)     | **Current** — all new contracts; `version` key always present |

> **Parsing rule:** If the `version` key is absent, treat the payload as v1. Always parse
> `amount` when present — it is the one field guaranteed across all value-transfer events in
> both generations.

---

## 2. Schema Versioning Policy

- A **major version bump** (e.g. v2 → v3) is required when a previously required field is
  removed or its type changes incompatibly.
- **Additive fields** (new optional keys) do not require a version bump; parsers must ignore
  unknown fields.
- The `version` integer is always the **first field** in a v2 payload struct.
- The constant `EVENT_VERSION_V2: u32 = 2` is defined in both `bounty_escrow/src/events.rs`
  and `program_escrow/src/lib.rs` and must always match this document.

---

## 3. How Events Are Emitted (Soroban Mechanics)

Soroban events are published via `env.events().publish(topics, data)`.

- **Topics** — a tuple of up to 4 `Symbol` values used for filtering.
- **Data** — a typed `#[contracttype]` struct (v2) or a raw tuple (some v1 events).

The two patterns used in this codebase:

```rust
// Pattern A — bounty_escrow/src/events.rs
// Per-bounty events include bounty_id in the topic for efficient filtering.
pub fn emit_funds_locked(env: &Env, event: FundsLocked) {
    let topics = (symbol_short!("f_lock"), event.bounty_id);
    env.events().publish(topics, event.clone());
}

// Pattern B — program_escrow/src/lib.rs
// Contract-wide events use a single-element topic tuple.
env.events().publish(
    (BATCH_PAYOUT,),
    BatchPayoutEvent { version: EVENT_VERSION_V2, .. },
);
```

> **Topic arity:** `bounty_escrow` uses **two-element topics** for per-bounty events
> (action + `bounty_id`) and **one-element topics** for contract-wide events.
> `program_escrow` and `grainlify-core` use **one-element topics** throughout.

---

## 4. How Events Are Consumed

### Via Stellar RPC (TypeScript/JS)

```typescript
import { SorobanRpc, xdr, scValToNative } from '@stellar/stellar-sdk';

const server = new SorobanRpc.Server('https://soroban-testnet.stellar.org');

const { events } = await server.getEvents({
  startLedger: fromLedger,
  filters: [{
    type: 'contract',
    contractIds: [CONTRACT_ID],
    topics: [['*']],  // wildcard; narrow with e.g. [['f_lock', '*']]
  }],
});

for (const ev of events) {
  const action  = ev.topic[0].value();       // e.g. "f_lock"
  const payload = scValToNative(ev.value);   // typed JS object
  console.log(action, payload);
}
```

### Via Trustless Work API

Set a `webhook_url` during contract initialisation. The API re-publishes deserialized payloads
as JSON over HTTPS POST / Server-Sent Events.

### Via SubQuery Indexer

See `spikes/subquery-indexer` in the Product repo for a SubQuery project that maps raw Soroban
events to a queryable GraphQL schema.

---

## 5. Contract: `bounty_escrow`

> **Source:** `contracts/bounty_escrow/src/events.rs`
> `EVENT_VERSION_V2: u32 = 2` is declared at the top of that file.

---

### 5.1 `BountyEscrowInitialized`

**Emitted by:** `emit_bounty_initialized()`
**Topics:** `(symbol_short!("init"),)`
**Struct:** `BountyEscrowInitialized`
**Lifecycle phase:** Contract initialisation

```rust
#[contracttype]
pub struct BountyEscrowInitialized {
    pub version:   u32,
    pub admin:     Address,
    pub token:     Address,
    pub timestamp: u64,
}
```

#### v2 Payload Example

```json
{
  "version":   2,
  "admin":     "GABC…",
  "token":     "GDQU…",
  "timestamp": 1740000000
}
```

| Field       | Rust type | v1 required | v2 required | Description |
|-------------|-----------|-------------|-------------|-------------|
| `version`   | `u32`     | No          | **Yes**     | Always `2` |
| `admin`     | `Address` | **Yes**     | **Yes**     | Contract administrator address |
| `token`     | `Address` | **Yes**     | **Yes**     | Token contract address used for escrow |
| `timestamp` | `u64`     | **Yes**     | **Yes**     | Ledger timestamp at initialisation |

---

### 5.2 `FundsLocked`

**Emitted by:** `emit_funds_locked()`
**Topics:** `(symbol_short!("f_lock"), event.bounty_id)`
**Struct:** `FundsLocked`
**Lifecycle phase:** Bounty funding

```rust
#[contracttype]
pub struct FundsLocked {
    pub version:   u32,
    pub bounty_id: u64,
    pub amount:    i128,
    pub depositor: Address,
    pub deadline:  u64,
}
```

#### v2 Payload Example

```json
{
  "version":   2,
  "bounty_id": 42,
  "amount":    1000000000,
  "depositor": "GABC…",
  "deadline":  1740086400
}
```

| Field       | Rust type | v1 required | v2 required | Description |
|-------------|-----------|-------------|-------------|-------------|
| `version`   | `u32`     | No          | **Yes**     | Always `2` |
| `bounty_id` | `u64`     | **Yes**     | **Yes**     | Bounty identifier; also in topic[1] for indexing |
| `amount`    | `i128`    | **Yes**     | **Yes**     | Amount locked (token stroops) |
| `depositor` | `Address` | **Yes**     | **Yes**     | Address that deposited the funds |
| `deadline`  | `u64`     | **Yes**     | **Yes**     | Unix timestamp after which funds may be refunded |

---

### 5.3 `FundsReleased`

**Emitted by:** `emit_funds_released()`
**Topics:** `(symbol_short!("f_rel"), event.bounty_id)`
**Struct:** `FundsReleased`
**Lifecycle phase:** Bounty completion / payout

```rust
#[contracttype]
pub struct FundsReleased {
    pub version:   u32,
    pub bounty_id: u64,
    pub amount:    i128,
    pub recipient: Address,
    pub timestamp: u64,
}
```

#### v2 Payload Example

```json
{
  "version":   2,
  "bounty_id": 42,
  "amount":    990000000,
  "recipient": "GDEF…",
  "timestamp": 1740100000
}
```

| Field       | Rust type | v1 required | v2 required | Description |
|-------------|-----------|-------------|-------------|-------------|
| `version`   | `u32`     | No          | **Yes**     | Always `2` |
| `bounty_id` | `u64`     | **Yes**     | **Yes**     | Bounty identifier; also in topic[1] |
| `amount`    | `i128`    | **Yes**     | **Yes**     | Net amount released to recipient |
| `recipient` | `Address` | **Yes**     | **Yes**     | Address receiving the released funds |
| `timestamp` | `u64`     | **Yes**     | **Yes**     | Ledger timestamp at release |

---

### 5.4 `FundsRefunded`

**Emitted by:** `emit_funds_refunded()`
**Topics:** `(symbol_short!("f_ref"), event.bounty_id)`
**Struct:** `FundsRefunded`
**Lifecycle phase:** Bounty cancellation / expired deadline

```rust
#[contracttype]
pub struct FundsRefunded {
    pub version:   u32,
    pub bounty_id: u64,
    pub amount:    i128,
    pub refund_to: Address,
    pub timestamp: u64,
}
```

#### v2 Payload Example

```json
{
  "version":   2,
  "bounty_id": 42,
  "amount":    1000000000,
  "refund_to": "GABC…",
  "timestamp": 1740200000
}
```

| Field       | Rust type | v1 required | v2 required | Description |
|-------------|-----------|-------------|-------------|-------------|
| `version`   | `u32`     | No          | **Yes**     | Always `2` |
| `bounty_id` | `u64`     | **Yes**     | **Yes**     | Bounty identifier; also in topic[1] |
| `amount`    | `i128`    | **Yes**     | **Yes**     | Full amount refunded |
| `refund_to` | `Address` | **Yes**     | **Yes**     | Address receiving the refund (original depositor) |
| `timestamp` | `u64`     | **Yes**     | **Yes**     | Ledger timestamp at refund |

---

### 5.5 `FeeCollected`

**Emitted by:** `emit_fee_collected()`
**Topics:** `(symbol_short!("fee"),)`
**Struct:** `FeeCollected`
**Lifecycle phase:** Any operation that deducts a fee (lock or release)

> **Version note:** `FeeCollected` does **not** carry a `version` field. It is a v1-style
> payload. Parsers must not require `version` for this event.

```rust
#[contracttype]
pub struct FeeCollected {
    pub operation_type: FeeOperationType,  // enum: Lock | Release
    pub amount:         i128,
    pub fee_rate:       i128,
    pub recipient:      Address,
    pub timestamp:      u64,
}

#[contracttype]
pub enum FeeOperationType { Lock, Release }
```

#### Payload Example

```json
{
  "operation_type": "Lock",
  "amount":         10000000,
  "fee_rate":       100,
  "recipient":      "GFEE…",
  "timestamp":      1740000100
}
```

| Field            | Rust type          | Required | Description |
|------------------|--------------------|----------|-------------|
| `operation_type` | `FeeOperationType` | **Yes**  | `Lock` or `Release` — which operation triggered the fee |
| `amount`         | `i128`             | **Yes**  | Fee amount collected (token stroops) |
| `fee_rate`       | `i128`             | **Yes**  | Fee rate in basis points at time of collection |
| `recipient`      | `Address`          | **Yes**  | Address that received the fee |
| `timestamp`      | `u64`              | **Yes**  | Ledger timestamp |

---

### 5.6 `BatchFundsLocked`

**Emitted by:** `emit_batch_funds_locked()`
**Topics:** `(symbol_short!("b_lock"),)`
**Struct:** `BatchFundsLocked`
**Lifecycle phase:** Batch bounty creation

> **Version note:** No `version` field — v1-style payload.

```rust
#[contracttype]
pub struct BatchFundsLocked {
    pub count:        u32,
    pub total_amount: i128,
    pub timestamp:    u64,
}
```

#### Payload Example

```json
{
  "count":        5,
  "total_amount": 5000000000,
  "timestamp":    1740000200
}
```

| Field          | Rust type | Required | Description |
|----------------|-----------|----------|-------------|
| `count`        | `u32`     | **Yes**  | Number of bounties locked in the batch |
| `total_amount` | `i128`    | **Yes**  | Aggregate amount locked across all bounties |
| `timestamp`    | `u64`     | **Yes**  | Ledger timestamp |

---

### 5.7 `BatchFundsReleased`

**Emitted by:** `emit_batch_funds_released()`
**Topics:** `(symbol_short!("b_rel"),)`
**Struct:** `BatchFundsReleased`
**Lifecycle phase:** Batch bounty payout

> **Version note:** No `version` field — v1-style payload.

```rust
#[contracttype]
pub struct BatchFundsReleased {
    pub count:        u32,
    pub total_amount: i128,
    pub timestamp:    u64,
}
```

#### Payload Example

```json
{
  "count":        5,
  "total_amount": 4900000000,
  "timestamp":    1740100200
}
```

| Field          | Rust type | Required | Description |
|----------------|-----------|----------|-------------|
| `count`        | `u32`     | **Yes**  | Number of bounties released in the batch |
| `total_amount` | `i128`    | **Yes**  | Aggregate net amount released |
| `timestamp`    | `u64`     | **Yes**  | Ledger timestamp |

---

### 5.8 `ApprovalAdded`

**Emitted by:** `emit_approval_added()`
**Topics:** `(symbol_short!("approval"), event.bounty_id)`
**Struct:** `ApprovalAdded`
**Lifecycle phase:** Work submission / approval flow

> **Version note:** No `version` field — v1-style payload.

```rust
#[contracttype]
pub struct ApprovalAdded {
    pub bounty_id:   u64,
    pub contributor: Address,
    pub approver:    Address,
    pub timestamp:   u64,
}
```

#### Payload Example

```json
{
  "bounty_id":   42,
  "contributor": "GCON…",
  "approver":    "GAPR…",
  "timestamp":   1740050000
}
```

| Field         | Rust type | Required | Description |
|---------------|-----------|----------|-------------|
| `bounty_id`   | `u64`     | **Yes**  | Bounty identifier; also in topic[1] |
| `contributor` | `Address` | **Yes**  | Address of the work submitter |
| `approver`    | `Address` | **Yes**  | Address of the approver |
| `timestamp`   | `u64`     | **Yes**  | Ledger timestamp |

---

### 5.9 `FeeConfigUpdated`

**Emitted by:** `emit_fee_config_updated()`
**Topics:** `(symbol_short!("fee_cfg"),)`
**Struct:** `FeeConfigUpdated`
**Lifecycle phase:** Admin fee configuration change

> **Version note:** No `version` field — v1-style payload.

```rust
#[contracttype]
pub struct FeeConfigUpdated {
    pub lock_fee_rate:    i128,
    pub release_fee_rate: i128,
    pub fee_recipient:    Address,
    pub fee_enabled:      bool,
    pub timestamp:        u64,
}
```

#### Payload Example

```json
{
  "lock_fee_rate":    50,
  "release_fee_rate": 100,
  "fee_recipient":    "GFEE…",
  "fee_enabled":      true,
  "timestamp":        1740000050
}
```

| Field              | Rust type | Required | Description |
|--------------------|-----------|----------|-------------|
| `lock_fee_rate`    | `i128`    | **Yes**  | New lock-operation fee in basis points |
| `release_fee_rate` | `i128`    | **Yes**  | New release-operation fee in basis points |
| `fee_recipient`    | `Address` | **Yes**  | Address that will receive fees going forward |
| `fee_enabled`      | `bool`    | **Yes**  | Whether fee collection is active after this update |
| `timestamp`        | `u64`     | **Yes**  | Ledger timestamp |

---

### 5.10 `PauseStateChanged` (bounty)

**Emitted by:** `emit_pause_state_changed()` — delegates to `crate::PauseStateChanged`
**Topics:** `(symbol_short!("pause"), event.operation.clone())`
**Struct:** `PauseStateChanged` (defined at crate root, shared with program_escrow)
**Lifecycle phase:** Admin pause / unpause

```rust
#[contracttype]
pub struct PauseStateChanged {
    pub operation: Symbol,   // e.g. symbol_short!("lock") / "release" / "refund"
    pub paused:    bool,
    pub admin:     Address,
}
```

#### Payload Example

```json
{
  "operation": "lock",
  "paused":    true,
  "admin":     "GADM…"
}
```

| Field       | Rust type | Required | Description |
|-------------|-----------|----------|-------------|
| `operation` | `Symbol`  | **Yes**  | Which operation was toggled: `lock`, `release`, or `refund` |
| `paused`    | `bool`    | **Yes**  | `true` = now paused, `false` = now unpaused |
| `admin`     | `Address` | **Yes**  | Admin address that triggered the change |

---

## 6. Contract: `program_escrow`

> **Source:** `contracts/program_escrow/src/lib.rs`
> The following constants are defined at the top of that file:

```rust
const PROGRAM_INITIALIZED: Symbol = symbol_short!("PrgInit");
const FUNDS_LOCKED:         Symbol = symbol_short!("FndsLock");
const BATCH_PAYOUT:         Symbol = symbol_short!("BatchPay");
const PAYOUT:               Symbol = symbol_short!("Payout");
const PAUSE_STATE_CHANGED:  Symbol = symbol_short!("PauseSt");
const EVENT_VERSION_V2:     u32    = 2;
```

---

### 6.1 `ProgramInitialized`

**Emitted by:** `initialize_program()` (also exposed as `init_program()`)
**Topics:** `(PROGRAM_INITIALIZED,)` → `("PrgInit",)`
**Struct:** `ProgramInitializedEvent`
**Lifecycle phase:** Program setup (one-time per contract instance)

```rust
#[contracttype]
pub struct ProgramInitializedEvent {
    pub version:               u32,
    pub program_id:            String,
    pub authorized_payout_key: Address,
    pub token_address:         Address,
    pub total_funds:           i128,   // always 0 at init
}
```

#### v2 Payload Example

```json
{
  "version":               2,
  "program_id":            "Hackathon2024",
  "authorized_payout_key": "GABC…",
  "token_address":         "GDQU…",
  "total_funds":           0
}
```

| Field                   | Rust type | v2 required | Description |
|-------------------------|-----------|-------------|-------------|
| `version`               | `u32`     | **Yes**     | Always `2` |
| `program_id`            | `String`  | **Yes**     | Unique program / hackathon identifier |
| `authorized_payout_key` | `Address` | **Yes**     | Backend address authorised to trigger payouts |
| `token_address`         | `Address` | **Yes**     | Token contract address used for transfers |
| `total_funds`           | `i128`    | **Yes**     | Always `0` at initialisation — funds are locked separately |

---

### 6.2 `FundsLocked` (program)

**Emitted by:** `lock_program_funds()`
**Topics:** `(FUNDS_LOCKED,)` → `("FndsLock",)`
**Struct:** `FundsLockedEvent`
**Lifecycle phase:** Prize pool funding (may be called multiple times, balance is cumulative)

```rust
#[contracttype]
pub struct FundsLockedEvent {
    pub version:           u32,
    pub program_id:        String,
    pub amount:            i128,
    pub remaining_balance: i128,
}
```

#### v2 Payload Example

```json
{
  "version":           2,
  "program_id":        "Hackathon2024",
  "amount":            100000000000,
  "remaining_balance": 100000000000
}
```

| Field               | Rust type | v2 required | Description |
|---------------------|-----------|-------------|-------------|
| `version`           | `u32`     | **Yes**     | Always `2` |
| `program_id`        | `String`  | **Yes**     | Program identifier |
| `amount`            | `i128`    | **Yes**     | Amount locked in this call (token stroops) |
| `remaining_balance` | `i128`    | **Yes**     | Program's new running balance after this lock |

---

### 6.3 `BatchPayout`

**Emitted by:** `batch_payout()`
**Topics:** `(BATCH_PAYOUT,)` → `("BatchPay",)`
**Struct:** `BatchPayoutEvent`
**Lifecycle phase:** Multi-winner prize distribution

```rust
#[contracttype]
pub struct BatchPayoutEvent {
    pub version:           u32,
    pub program_id:        String,
    pub recipient_count:   u32,
    pub total_amount:      i128,
    pub remaining_balance: i128,
}
```

#### v2 Payload Example

```json
{
  "version":           2,
  "program_id":        "Hackathon2024",
  "recipient_count":   3,
  "total_amount":      100000000000,
  "remaining_balance": 0
}
```

| Field               | Rust type | v2 required | Description |
|---------------------|-----------|-------------|-------------|
| `version`           | `u32`     | **Yes**     | Always `2` |
| `program_id`        | `String`  | **Yes**     | Program identifier |
| `recipient_count`   | `u32`     | **Yes**     | Number of recipients paid in this batch call |
| `total_amount`      | `i128`    | **Yes**     | Aggregate amount disbursed (token stroops) |
| `remaining_balance` | `i128`    | **Yes**     | Program balance after this batch |

---

### 6.4 `Payout`

**Emitted by:** `single_payout()`
**Topics:** `(PAYOUT,)` → `("Payout",)`
**Struct:** `PayoutEvent`
**Lifecycle phase:** Single-winner prize distribution

```rust
#[contracttype]
pub struct PayoutEvent {
    pub version:           u32,
    pub program_id:        String,
    pub recipient:         Address,
    pub amount:            i128,
    pub remaining_balance: i128,
}
```

#### v2 Payload Example

```json
{
  "version":           2,
  "program_id":        "Hackathon2024",
  "recipient":         "GWINNER…",
  "amount":            50000000000,
  "remaining_balance": 50000000000
}
```

| Field               | Rust type | v2 required | Description |
|---------------------|-----------|-------------|-------------|
| `version`           | `u32`     | **Yes**     | Always `2` |
| `program_id`        | `String`  | **Yes**     | Program identifier |
| `recipient`         | `Address` | **Yes**     | Winner's address |
| `amount`            | `i128`    | **Yes**     | Amount paid to this recipient |
| `remaining_balance` | `i128`    | **Yes**     | Program balance after this payout |

---

### 6.5 `PauseStateChanged` (program)

**Emitted by:** `set_paused()` — one event published **per changed flag**
**Topics:** `(PAUSE_STATE_CHANGED,)` → `("PauseSt",)`
**Data:** Raw tuple `(Symbol, bool, Address)` — not a named struct in this contract
**Lifecycle phase:** Admin pause / unpause

```rust
// Inside set_paused(), for each modified flag:
env.events().publish(
    (PAUSE_STATE_CHANGED,),
    (symbol_short!("lock"), paused, admin.clone()),  // or "release" / "refund"
);
```

#### Payload (raw tuple)

```json
["lock", true, "GADM…"]
```

| Tuple index | Rust type | Description |
|-------------|-----------|-------------|
| `0`         | `Symbol`  | Operation toggled: `lock`, `release`, or `refund` |
| `1`         | `bool`    | `true` = now paused, `false` = now unpaused |
| `2`         | `Address` | Admin address that triggered the change |

> **Compatibility note:** `bounty_escrow` publishes `PauseStateChanged` as a **named struct**
> with fields `{ operation, paused, admin }`. `program_escrow` publishes a **raw positional
> tuple**. Indexers must handle both representations.

---

## 7. Contract: `grainlify-core`

> **Source:** `contracts/grainlify-core/src/lib.rs`
> Events come from two places: the top-level `GrainlifyContract` impl (via
> `emit_migration_event`) and the inner `monitoring` module.

---

### 7.1 `MigrationEvent`

**Emitted by:** `emit_migration_event()` — called from `migrate()` on both success and failure paths
**Topics:** `(symbol_short!("migration"),)`
**Struct:** `MigrationEvent`
**Lifecycle phase:** Contract state migration (upgrade post-processing)

```rust
#[contracttype]
pub struct MigrationEvent {
    pub from_version:   u32,
    pub to_version:     u32,
    pub timestamp:      u64,
    pub migration_hash: BytesN<32>,
    pub success:        bool,
    pub error_message:  Option<String>,
}
```

#### Payload Example (success)

```json
{
  "from_version":   2,
  "to_version":     3,
  "timestamp":      1740300000,
  "migration_hash": "0101…(32 bytes hex)",
  "success":        true,
  "error_message":  null
}
```

#### Payload Example (failure — emitted before panic)

```json
{
  "from_version":   3,
  "to_version":     2,
  "timestamp":      1740300001,
  "migration_hash": "0101…",
  "success":        false,
  "error_message":  "Target version must be greater than current version"
}
```

| Field            | Rust type        | Required | Description |
|------------------|------------------|----------|-------------|
| `from_version`   | `u32`            | **Yes**  | Contract version before this migration attempt |
| `to_version`     | `u32`            | **Yes**  | Intended target version |
| `timestamp`      | `u64`            | **Yes**  | Ledger timestamp |
| `migration_hash` | `BytesN<32>`     | **Yes**  | Caller-supplied hash for external audit verification |
| `success`        | `bool`           | **Yes**  | `true` = migration completed; `false` = validation failed |
| `error_message`  | `Option<String>` | **Yes**  | Failure reason string; `null` on success |

---

### 7.2 `OperationMetric`

**Emitted by:** `monitoring::track_operation()` — called from `init_admin()`, `upgrade()`,
`set_version()`, and `migrate()`
**Topics:** `(symbol_short!("metric"), symbol_short!("op"))`
**Struct:** `monitoring::OperationMetric`
**Lifecycle phase:** Every admin operation (audit trail / observability)

```rust
#[contracttype]
pub struct OperationMetric {
    pub operation: Symbol,
    pub caller:    Address,
    pub timestamp: u64,
    pub success:   bool,
}
```

#### Payload Example

```json
{
  "operation": "upgrade",
  "caller":    "GADM…",
  "timestamp": 1740300000,
  "success":   true
}
```

| Field       | Rust type | Required | Description |
|-------------|-----------|----------|-------------|
| `operation` | `Symbol`  | **Yes**  | Short name: `init`, `upgrade`, `set_ver`, `migrate` |
| `caller`    | `Address` | **Yes**  | Address that invoked the operation |
| `timestamp` | `u64`     | **Yes**  | Ledger timestamp |
| `success`   | `bool`    | **Yes**  | Whether the operation completed without panic |

---

### 7.3 `PerformanceMetric`

**Emitted by:** `monitoring::emit_performance()` — called after every admin operation
**Topics:** `(symbol_short!("metric"), symbol_short!("perf"))`
**Struct:** `monitoring::PerformanceMetric`
**Lifecycle phase:** Every admin operation (latency / performance observability)

```rust
#[contracttype]
pub struct PerformanceMetric {
    pub function:  Symbol,
    pub duration:  u64,
    pub timestamp: u64,
}
```

#### Payload Example

```json
{
  "function":  "upgrade",
  "duration":  0,
  "timestamp": 1740300000
}
```

| Field       | Rust type | Required | Description |
|-------------|-----------|----------|-------------|
| `function`  | `Symbol`  | **Yes**  | Function name: `init`, `upgrade`, `set_ver`, `migrate` |
| `duration`  | `u64`     | **Yes**  | Elapsed ledger-time seconds (will be `0` in test env where timestamp doesn't advance) |
| `timestamp` | `u64`     | **Yes**  | Ledger timestamp at end of operation |

---

## 8. Event Topic Reference

Complete lookup table of every `env.events().publish(topics, …)` call in this codebase:

| Contract         | Topics (Rust literal)                                   | Resolved strings          | Data type                 |
|------------------|---------------------------------------------------------|---------------------------|---------------------------|
| `bounty_escrow`  | `(symbol_short!("init"),)`                              | `"init"`                  | `BountyEscrowInitialized` |
| `bounty_escrow`  | `(symbol_short!("f_lock"), bounty_id)`                  | `"f_lock"` + u64          | `FundsLocked`             |
| `bounty_escrow`  | `(symbol_short!("f_rel"), bounty_id)`                   | `"f_rel"` + u64           | `FundsReleased`           |
| `bounty_escrow`  | `(symbol_short!("f_ref"), bounty_id)`                   | `"f_ref"` + u64           | `FundsRefunded`           |
| `bounty_escrow`  | `(symbol_short!("fee"),)`                               | `"fee"`                   | `FeeCollected`            |
| `bounty_escrow`  | `(symbol_short!("b_lock"),)`                            | `"b_lock"`                | `BatchFundsLocked`        |
| `bounty_escrow`  | `(symbol_short!("b_rel"),)`                             | `"b_rel"`                 | `BatchFundsReleased`      |
| `bounty_escrow`  | `(symbol_short!("approval"), bounty_id)`                | `"approval"` + u64        | `ApprovalAdded`           |
| `bounty_escrow`  | `(symbol_short!("fee_cfg"),)`                           | `"fee_cfg"`               | `FeeConfigUpdated`        |
| `bounty_escrow`  | `(symbol_short!("pause"), event.operation.clone())`     | `"pause"` + op symbol     | `PauseStateChanged`       |
| `program_escrow` | `(PROGRAM_INITIALIZED,)` = `("PrgInit",)`               | `"PrgInit"`               | `ProgramInitializedEvent` |
| `program_escrow` | `(FUNDS_LOCKED,)` = `("FndsLock",)`                     | `"FndsLock"`              | `FundsLockedEvent`        |
| `program_escrow` | `(BATCH_PAYOUT,)` = `("BatchPay",)`                     | `"BatchPay"`              | `BatchPayoutEvent`        |
| `program_escrow` | `(PAYOUT,)` = `("Payout",)`                             | `"Payout"`                | `PayoutEvent`             |
| `program_escrow` | `(PAUSE_STATE_CHANGED,)` = `("PauseSt",)`               | `"PauseSt"`               | Tuple `(Symbol,bool,Address)` |
| `grainlify-core` | `(symbol_short!("migration"),)`                         | `"migration"`             | `MigrationEvent`          |
| `grainlify-core` | `(symbol_short!("metric"), symbol_short!("op"))`        | `"metric"` + `"op"`       | `OperationMetric`         |
| `grainlify-core` | `(symbol_short!("metric"), symbol_short!("perf"))`      | `"metric"` + `"perf"`     | `PerformanceMetric`       |

---

## 9. Payload Field Reference

All fields appearing across all three contracts:

| Field                   | Rust type          | XDR Val     | Contracts using it |
|-------------------------|--------------------|-------------|---------------------|
| `version`               | `u32`              | `U32`       | bounty (v2 events), program |
| `admin`                 | `Address`          | `Address`   | bounty |
| `token` / `token_address` | `Address`        | `Address`   | bounty, program |
| `timestamp`             | `u64`              | `U64`       | bounty, program, grainlify |
| `bounty_id`             | `u64`              | `U64`       | bounty (also topic[1]) |
| `amount`                | `i128`             | `I128`      | bounty, program |
| `depositor`             | `Address`          | `Address`   | bounty |
| `deadline`              | `u64`              | `U64`       | bounty |
| `recipient`             | `Address`          | `Address`   | bounty, program |
| `refund_to`             | `Address`          | `Address`   | bounty |
| `operation_type`        | `FeeOperationType` | `Map/Enum`  | bounty |
| `fee_rate`              | `i128`             | `I128`      | bounty |
| `lock_fee_rate`         | `i128`             | `I128`      | bounty |
| `release_fee_rate`      | `i128`             | `I128`      | bounty |
| `fee_recipient`         | `Address`          | `Address`   | bounty |
| `fee_enabled`           | `bool`             | `Bool`      | bounty |
| `count`                 | `u32`              | `U32`       | bounty |
| `total_amount`          | `i128`             | `I128`      | bounty, program |
| `contributor`           | `Address`          | `Address`   | bounty |
| `approver`              | `Address`          | `Address`   | bounty |
| `operation`             | `Symbol`           | `Symbol`    | bounty, program |
| `paused`                | `bool`             | `Bool`      | bounty, program |
| `program_id`            | `String`           | `String`    | program |
| `authorized_payout_key` | `Address`          | `Address`   | program |
| `total_funds`           | `i128`             | `I128`      | program |
| `remaining_balance`     | `i128`             | `I128`      | program |
| `recipient_count`       | `u32`              | `U32`       | program |
| `from_version`          | `u32`              | `U32`       | grainlify |
| `to_version`            | `u32`              | `U32`       | grainlify |
| `migration_hash`        | `BytesN<32>`       | `Bytes`     | grainlify |
| `success`               | `bool`             | `Bool`      | grainlify |
| `error_message`         | `Option<String>`   | `Void/String` | grainlify |
| `caller`                | `Address`          | `Address`   | grainlify |
| `function`              | `Symbol`           | `Symbol`    | grainlify |
| `duration`              | `u64`              | `U64`       | grainlify |

> **Integer precision:** `i128` values must be handled as `BigInt` in JavaScript/TypeScript.
> USDC on Stellar uses 7 decimal places (1 USDC = 10,000,000 stroops).

---

## 10. v1 → v2 Migration Guide

Five events in `bounty_escrow` are **permanently v1** (no `version` field was ever added):
`FeeCollected`, `BatchFundsLocked`, `BatchFundsReleased`, `ApprovalAdded`, `FeeConfigUpdated`.
Parsers must never require `version` on these events.

For events that do carry `version` (`BountyEscrowInitialized`, `FundsLocked`, `FundsReleased`,
`FundsRefunded` in bounty_escrow; all events in `program_escrow`):

1. **Detect version.** Read `version` from payload. If absent → treat as `1`.
2. **`amount` is always safe.** Both v1 and v2 include `amount` on value-transfer events.
3. **New v2 fields are additive.** Initialise absent keys as `null` / `None`.
4. **Do not rely on field order.** `scValToNative` returns objects keyed by name; iterate
   by key name, not position.
5. **Ignore unknown fields.** Future minor versions will add keys without bumping `version`.

---

## 11. Forward-Compatible Parsing

```typescript
import { scValToNative, xdr } from '@stellar/stellar-sdk';

interface BaseEscrowEvent {
  version?: number;          // absent = v1
  amount?:  bigint;          // present on all value-transfer events
  [key: string]: unknown;    // absorb future additive fields
}

function parseContractEvent(rawVal: xdr.ScVal): BaseEscrowEvent {
  const native = scValToNative(rawVal) as Record<string, unknown>;
  return {
    ...native,                // keep all fields, including future ones
    version: native['version'] !== undefined ? Number(native['version']) : undefined,
    amount:  native['amount']  !== undefined ? BigInt(native['amount'] as string) : undefined,
  };
}

// Narrow to a specific event by checking for discriminating fields:
function isFundsLocked(e: BaseEscrowEvent): boolean {
  return 'bounty_id' in e && 'depositor' in e && 'deadline' in e;
}
function isBatchPayout(e: BaseEscrowEvent): boolean {
  return 'recipient_count' in e && 'remaining_balance' in e;
}
```

---

## 12. Security Notes

- **No secrets on-chain.** Events contain public keys and amounts only. No private keys,
  API tokens, or off-chain credentials are ever emitted.
- **Event authenticity.** Stellar protocol verifies the publishing contract ID matches the
  invoked contract. Event spoofing at the protocol level is not possible.
- **Reentrancy protection.** `program_escrow` wraps `batch_payout()`, `single_payout()`, and
  `trigger_program_releases()` with `reentrancy_guard::check_not_entered()` /
  `set_entered()` / `clear_entered()`. Events are only emitted after all state mutations and
  token transfers complete — before the guard is cleared. A partially-executed call cannot
  produce a misleadingly successful event.
- **Pause guard.** Both `bounty_escrow` and `program_escrow` check per-operation pause flags
  before executing. Events are **not** emitted for calls that are rejected by the pause guard.
- **`PauseStateChanged` is emitted for every flag individually.** A single call to
  `set_paused(lock: true, release: true, refund: false)` emits two separate events.
- **Amount validation.** Contract logic validates `amount > 0` and balance sufficiency before
  emitting. Consuming systems should still assert defensively.
- **`i128` overflow safety.** `program_escrow` uses `checked_add` / `checked_mul` throughout
  payout logic. Consuming systems in languages without native 128-bit integers must use `BigInt`.
- **`migration_hash` is caller-supplied.** `MigrationEvent.migration_hash` is not validated
  on-chain. Treat it as an opaque external audit reference, not a cryptographic guarantee.

---

## 13. Test Coverage Notes

Events are exercised by tests embedded in each source file. Key scenarios per contract:

| Test name | Contract | Event(s) validated |
|---|---|---|
| `test_register_single_program` | program_escrow | `ProgramInitialized` — all v2 fields present |
| `test_lock_funds_single_program` | program_escrow | `FundsLocked` — amount and remaining_balance |
| `test_lock_funds_cumulative` | program_escrow | `FundsLocked` — emitted three times, balance accumulates |
| `test_batch_payout_mismatched_lengths` | program_escrow | No event — panics before emission |
| `test_batch_payout_insufficient_balance` | program_escrow | No event — panics before emission |
| `test_anti_abuse_config_update` | program_escrow | `PauseStateChanged` indirectly via `set_paused` |
| `test_complete_upgrade_and_migration_workflow` | grainlify | `MigrationEvent` + `OperationMetric` + `PerformanceMetric` |
| `test_migration_v1_to_v2` | grainlify | `MigrationEvent` success — from/to versions correct |
| `test_migration_invalid_target_version` | grainlify | `MigrationEvent` failure — emitted before panic |
| `test_migration_idempotency` | grainlify | `MigrationEvent` emitted exactly once on repeat call |
| `test_migration_emits_success_event` | grainlify | Event count increases after `migrate()` |
| `test_migration_requires_admin_authorization` | grainlify | Auth check fires before event |
| `test_migration_only_runs_once_per_version` | grainlify | `MigrationEvent` timestamp unchanged on second call |

Run all tests:

```bash
cd contracts/bounty_escrow  && cargo test -- --nocapture
cd contracts/program_escrow && cargo test -- --nocapture
cd contracts/grainlify-core && cargo test -- --nocapture
```

Measure line coverage (target: **≥ 95%** on event-emitting paths):

```bash
cargo tarpaulin --out Html --output-dir coverage/
```

---

## 14. Inline Source References

| Event | Emitting function | Source file |
|---|---|---|
| `BountyEscrowInitialized` | `emit_bounty_initialized()` | `contracts/bounty_escrow/src/events.rs` line ~13 |
| `FundsLocked` (bounty) | `emit_funds_locked()` | `contracts/bounty_escrow/src/events.rs` line ~26 |
| `FundsReleased` (bounty) | `emit_funds_released()` | `contracts/bounty_escrow/src/events.rs` line ~39 |
| `FundsRefunded` | `emit_funds_refunded()` | `contracts/bounty_escrow/src/events.rs` line ~52 |
| `FeeCollected` | `emit_fee_collected()` | `contracts/bounty_escrow/src/events.rs` line ~79 |
| `BatchFundsLocked` | `emit_batch_funds_locked()` | `contracts/bounty_escrow/src/events.rs` line ~91 |
| `FeeConfigUpdated` | `emit_fee_config_updated()` | `contracts/bounty_escrow/src/events.rs` line ~104 |
| `BatchFundsReleased` | `emit_batch_funds_released()` | `contracts/bounty_escrow/src/events.rs` line ~118 |
| `ApprovalAdded` | `emit_approval_added()` | `contracts/bounty_escrow/src/events.rs` line ~131 |
| `PauseStateChanged` (bounty) | `emit_pause_state_changed()` | `contracts/bounty_escrow/src/events.rs` line ~153 |
| `ProgramInitialized` | `initialize_program()` | `contracts/program_escrow/src/lib.rs` – end of `initialize_program` fn |
| `FundsLocked` (program) | `lock_program_funds()` | `contracts/program_escrow/src/lib.rs` – end of `lock_program_funds` fn |
| `BatchPayout` | `batch_payout()` | `contracts/program_escrow/src/lib.rs` – end of `batch_payout` fn |
| `Payout` | `single_payout()` | `contracts/program_escrow/src/lib.rs` – end of `single_payout` fn |
| `PauseStateChanged` (program) | `set_paused()` | `contracts/program_escrow/src/lib.rs` – inside `set_paused`, per-flag branch |
| `MigrationEvent` | `emit_migration_event()` → `migrate()` | `contracts/grainlify-core/src/lib.rs` – success and failure paths in `migrate` |
| `OperationMetric` | `monitoring::track_operation()` | `contracts/grainlify-core/src/lib.rs` – monitoring module, called from `init_admin`, `upgrade`, `set_version`, `migrate` |
| `PerformanceMetric` | `monitoring::emit_performance()` | `contracts/grainlify-core/src/lib.rs` – monitoring module, called from same admin fns |

---

## 15. Changelog

| Date       | Doc version | Branch / Author           | Notes |
|------------|-------------|---------------------------|-------|
| 2026-03-03 | 2.0.0       | `docs/event-schema-audit` | Full source-grounded audit against `bounty_escrow/src/events.rs`, `program_escrow/src/lib.rs`, and `grainlify-core/src/lib.rs`. Replaced previously inferred schema with exact `#[contracttype]` struct definitions, correct topic tuples, v1/v2 versioning per-event, complete topic reference table, reentrancy/pause security notes, tarpaulin command, and forward-compatible TypeScript parser. |
| (prior)    | 1.0.0       | —                         | Initial placeholder schema |