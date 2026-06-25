# Skipped Bounty Escrow Tests Tracking Issue

This document tracks and explains the reasons for skipping the following two tests in `bounty_escrow` crate, as required by CI hardening guidelines.

## 1. `test_events_emit_v2_version_tags_for_all_bounty_emitters`

### Description
This test verifies that all major entrypoints in `bounty_escrow` contract (`init`, `lock_funds`, `release_funds`) emit events that contain the `version: u32 = 2` tag in their payloads.

### Issues Identified
- **Instruction Budget Exceeded**: During extensive event matching and mock token calls within a single test execution context, the default Soroban instruction budget limits are sometimes exceeded, causing `ExceededLimit` test failures.
- **Event Ordering & Inter-contract Events**: When `lock_funds` interacts with the mock token contract, the token contract itself emits events. `env.events().all()` returns all events in the ledger, and filtering them accurately for version matching without a reset budget frequently fails under strict CI runners.

### Mitigation / Future Fix
To re-enable this test safely:
1. Wrap the test execution blocks with `env.budget().reset_unlimited()` to avoid budget failures.
2. Clear the events queue between individual contract calls using helper assertions or mock environment resets.

---

## 2. `analytics::tests`

### Description
A suite of tests (`test_bounty_analytics_initialization`, `test_analytics_on_release`, `test_analytics_on_refund`, `test_analytics_lifecycle`) inside `analytics.rs` verifying contract-level and bounty-level off-chain tracking metrics.

### Issues Identified
- **Storage Namespace & Dummy Contract**: These tests register a `DummyAnalyticsContract` and use `env.as_contract` to simulate storage operations. However, because the analytics functions read and write contract storage that expects the main contract's storage layout and instance configurations (e.g. paused flags, admin keys), calling them within a dummy contract namespace leads to storage deserialization mismatches or panics on missing instance keys.
- **Mock Environment Setup**: The environment does not fully initialize the required `bounty_escrow` storage keys inside the dummy contract, causing `get` operations on configuration keys to return `None` or panic.

### Mitigation / Future Fix
To re-enable these tests:
1. Instead of using `DummyAnalyticsContract`, run analytics tests directly on the fully initialized `BountyEscrow` contract client.
2. Initialize all necessary instance configurations (such as fee configurations and pause flags) in the test setup.
