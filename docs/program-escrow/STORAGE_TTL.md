# Program Escrow Storage TTL Strategy

This document outlines the TTL (Time To Live) extension strategy for the Program Escrow contract's persistent storage.

## Overview

Soroban persistent storage entries can be archived if their TTL is not extended. In the Program Escrow contract, several key data structures must persist for long periods:

- `PROGRAM_DATA`: Core configuration and payout history.
- `SCHEDULES`: Future release schedules, which may be set months or years in advance.
- `RELEASE_HISTORY`: Audit trail of all released funds.

If these entries are archived, future payouts could become untriggerable until the storage is manually restored.

## TTL Constants

The following constants are defined in `program-escrow/src/lib.rs` to manage persistent and instance storage lifetimes:

- `PERSISTENT_TTL_THRESHOLD`: `17,280` ledgers (approx. 1 day on 5s ledgers). Bumping occurs when the remaining TTL falls below this threshold.
- `PERSISTENT_TTL_EXTEND_TO`: `518,400` ledgers (approx. 30 days on 5s ledgers). Entries are extended to this horizon.

## Strategy

To ensure data longevity and fund safety, the contract implements a "bump-on-access" strategy for both persistent and instance storage:

1. **Write Paths**: Every operation that creates or updates a persistent or instance entry (e.g., `initialize_program`, `lock_program_funds`, `create_program_release_schedule`, `set_paused`) automatically extends the TTL.
2. **Read Paths (Critical)**: Functions involved in triggering releases, querying history, or checking admin status also perform TTL extension.
3. **Periodic Maintenance**: The `trigger_program_releases` function is designed to be called periodically (e.g., by an automation service). Each call re-bumps the TTL for the `SCHEDULES` vector, even if no schedules are currently due. This keeps future schedules alive indefinitely as long as the contract remains active.

## Schedule Horizon vs. TTL

While the TTL extension horizon is set to ~30 days, schedules can be created for any time in the future. The safety assumption is that:
- The contract is interacted with at least once every 30 days (either via a new lock, a release, or a periodic call to `trigger_program_releases`).
- Each interaction refreshes the 30-day window.

If a contract is expected to remain completely idle for longer than 30 days with future schedules pending, an external bot should be configured to call a read-only query or `trigger_program_releases` periodically to maintain the storage TTL.
