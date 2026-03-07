# Circuit Breaker for Bounty Escrow

## Overview

This document describes the circuit breaker implementation for the Bounty Escrow contract, which provides automatic failure detection and protection against cascading failures during fund operations.

## Architecture

The circuit breaker follows a three-state pattern adapted from the program-escrow implementation:

```
  [Closed] ──(failure_count >= threshold)──> [Open]
     ^                                          │
     │                                          │
  (reset by admin)                    (stays open until reset)
     │                                          │
  [HalfOpen] <────────────────────────────────┘
                   (admin calls reset)
```

### Circuit States

- **Closed**: Normal operation. All protected operations are allowed.
- **Open**: Circuit is open; all protected operations are rejected immediately without attempting execution.
- **HalfOpen**: Admin-initiated reset attempt. Allows operations to test if the system has recovered.

## Implementation Details

### Files Added/Modified

1. **`error_recovery.rs`** - New module containing:
   - Circuit breaker state types (`CircuitState`, `CircuitBreakerKey`)
   - Configuration struct (`CircuitBreakerConfig`)
   - Core functions: `check_and_allow`, `record_success`, `record_failure`
   - State transitions: `open_circuit`, `close_circuit`, `half_open_circuit`
   - Admin controls: `reset_circuit_breaker`, `set_circuit_admin`
   - Error logging and status queries

2. **`lib.rs`** - Modified to:
   - Import circuit breaker module and types
   - Add `CircuitBreakerOpen` error variant
   - Add circuit breaker checks to all fund-transferring operations
   - Add admin control functions

3. **`test_circuit_breaker.rs`** - Comprehensive test suite (33 tests)

### Protected Operations

The following operations are protected by the circuit breaker:
- `lock_funds` - Locks funds for a bounty
- `release_funds` - Releases funds to a contributor
- `claim` - Claims authorized funds within claim window
- `refund` - Refunds funds to the depositor
- `partial_release` - Partially releases locked funds
- `batch_lock_funds` - Batch lock funds operation
- `batch_release_funds` - Batch release funds operation

## Admin Controls

### Setting the Circuit Breaker Admin

```rust
pub fn set_circuit_breaker_admin(env: Env, admin: Address) -> Result<(), Error>
```

Sets the admin address that can reset the circuit breaker when it's open.

### Configuring Circuit Breaker Thresholds

```rust
pub fn set_circuit_breaker_config(
    env: Env,
    failure_threshold: u32,  // Failures needed to open circuit
    success_threshold: u32,    // Successes in HalfOpen needed to close
    max_error_log: u32,        // Max error log entries to retain
) -> Result<(), Error>
```

### Getting Circuit Status

```rust
pub fn get_circuit_breaker_status(env: Env) -> CircuitBreakerStatus
pub fn get_circuit_breaker_config(env: Env) -> CircuitBreakerConfig
pub fn get_circuit_breaker_admin(env: Env) -> Option<Address>
pub fn get_circuit_error_log(env: Env) -> Vec<ErrorEntry>
```

### Resetting the Circuit

```rust
pub fn reset_circuit(env: Env, admin: Address) -> Result<(), Error>
```

Transitions:
- `Open` -> `HalfOpen` (first reset)
- `HalfOpen` -> `Closed` (second reset or sufficient successes)

## Default Configuration

```rust
failure_threshold: 3     // Open after 3 consecutive failures
success_threshold: 1     // Close after 1 success in HalfOpen
max_error_log: 10        // Keep last 10 error entries
```

## Error Logging

The circuit breaker maintains an error log with:
- Operation type (e.g., "lock", "release")
- Bounty ID
- Error code
- Timestamp
- Failure count at time of error

## Security Considerations

1. **Admin Authorization**: All admin operations require authentication.
2. **State Transitions**: Circuit can only be reset by the registered admin.
3. **Automatic Opening**: Circuit opens automatically when failure threshold is reached.
4. **Manual Reset**: Circuit requires manual admin intervention to transition from Open.

## Testing

The circuit breaker includes 33 comprehensive tests covering:
- Initial state validation
- Failure threshold behavior
- State transitions (Closed -> Open -> HalfOpen -> Closed)
- Admin controls (set admin, update admin, unauthorized access)
- Configuration management
- Error logging
- Timestamp recording
- Integration with escrow operations

### Running Tests

```bash
cargo test --package bounty-escrow -- circuit_breaker
```

## Integration with Existing Systems

### Works with Pause Mechanism
The circuit breaker operates alongside the existing pause mechanism:
- Pause is for planned maintenance
- Circuit breaker is for automatic failure protection

### Works with Reentrancy Guard
Circuit breaker checks occur before reentrancy guard checks in protected operations.

## Event Emissions

The circuit breaker emits events for state changes:
- `cb_fail` - When a failure is recorded
- `cb_open` - When circuit opens
- `cb_half` - When circuit enters HalfOpen
- `cb_close` - When circuit closes
- `cb_reject` - When operation is rejected due to open circuit

## Future Enhancements

Potential improvements could include:
- Automatic reset after timeout
- Per-operation-type circuit breakers
- Configurable cooldown periods
- Integration with monitoring/alerting systems
