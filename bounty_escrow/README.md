# Bounty Escrow Contract

A Soroban smart contract for managing bounty escrow funds on the Stellar network.

## Documentation

Detailed documentation for this contract is available in the [`docs/`](../docs/index.md) directory:

- [Security](../docs/bounty_escrow/SECURITY.md)
- [Circuit Breaker](../docs/bounty_escrow/CIRCUIT_BREAKER.md)
- [Analytics](../docs/bounty_escrow/ANALYTICS_DOCUMENTATION.md)
- [Implementation Checklist](../docs/bounty_escrow/IMPLEMENTATION_CHECKLIST.md)
- [Feature Completion Report](../docs/bounty_escrow/FEATURE_COMPLETION_REPORT.md)
- [Analytics Implementation Summary](../docs/bounty_escrow/ANALYTICS_IMPLEMENTATION_SUMMARY.md)
- [Auto Refund Tests](../docs/bounty_escrow/contracts/escrow/AUTO_REFUND_TESTS.md)
- [CI Checks Summary](../docs/bounty_escrow/contracts/escrow/CI_CHECKS_SUMMARY.md)

---

# Soroban Project

## Project Structure

This repository uses the recommended structure for a Soroban project:
```text
.
├── contracts
│   └── hello_world
│       ├── src
│       │   ├── lib.rs
│       │   └── test.rs
│       └── Cargo.toml
├── Cargo.toml
└── README.md
```

- New Soroban contracts can be put in `contracts`, each in their own directory. There is already a `hello_world` contract in there to get you started.
- If you initialized this project with any other example contracts via `--with-example`, those contracts will be in the `contracts` directory as well.
- Contracts should have their own `Cargo.toml` files that rely on the top-level `Cargo.toml` workspace for their dependencies.
- Frontend libraries can be added to the top-level directory as well. If you initialized this project with a frontend template via `--frontend-template` you will have those files already included.

## Expiry Refund Sweep

`sweep_expired_refunds(bounty_ids)` processes a bounded batch of expired bounty escrows. Each entry must already be at or past its refund deadline, not blocked by any pending claim, and still in `Locked` or `PartiallyRefunded` state. The function validates the whole batch before moving funds, refunds each remaining balance to the recorded depositor, emits `BountyExpired` followed by `FundsRefunded`, and returns the number of swept bounties.

The sweep uses the same `MAX_BATCH_SIZE` limit as other batch escrow operations and is blocked by the refund pause flag, circuit breaker, and reentrancy guard.

## Property Testing

The `bounty-escrow` crate includes bounded `proptest` coverage for randomized lifecycle sequences. The property suite drives the real generated contract client through `lock_funds`, `partial_release`, `approve_refund`, `refund`, and `release_funds`, then checks escrow accounting, aggregate-state counts, token balances, and contract balance after each successful operation.

Run the property suite with:

```bash
cargo test -p bounty-escrow proptest_invariants
```

The proptest configuration uses a fixed case budget and capped shrinking so CI gets randomized coverage without unbounded runtime.
