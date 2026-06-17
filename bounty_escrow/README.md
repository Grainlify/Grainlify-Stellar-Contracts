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
