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

## Property Testing

The `bounty-escrow` crate includes bounded `proptest` coverage for randomized lifecycle sequences. The property suite drives the real generated contract client through `lock_funds`, `partial_release`, `approve_refund`, `refund`, and `release_funds`, then checks escrow accounting, aggregate-state counts, token balances, and contract balance after each successful operation.

Run the property suite with:

```bash
cargo test -p bounty-escrow proptest_invariants
```

The proptest configuration uses a fixed case budget and capped shrinking so CI gets randomized coverage without unbounded runtime.
