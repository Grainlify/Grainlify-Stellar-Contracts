# Program Escrow Whitelist - Implementation Summary

## Task Completed
Implemented a secure, configurable whitelist storage and enforcement mechanism to restrict single and batch payouts to whitelisted recipients when enforcement is enabled.

## Branch
`fix/program-escrow-whitelist`

## Changes Made

### 1. Storage & Key Definitions
Added the following variants to the `DataKey` enum in `program-escrow/src/lib.rs`:
- `Whitelist(Address)`: Stores the whitelist status (`bool`) for a specific recipient address in the contract's instance storage.
- `WhitelistEnforced`: Stores the global toggle status (`bool`) for whitelist enforcement in the contract's instance storage.

### 2. Event Types & Structures
Added the following event types and structures to support real-time monitoring of whitelist modifications:
- `WHITELIST_CHANGED` (`WlChange`): Emitted when an address is added to or removed from the whitelist.
  - Fields: `address` (Address), `whitelisted` (bool)
- `WHITELIST_ENFORCEMENT_CHANGED` (`WlEnfChg`): Emitted when the whitelist enforcement flag is toggled.
  - Fields: `enabled` (bool)

### 3. Public Entrypoints / Views
Implemented the following public functions on `ProgramEscrowContract`:
- `set_whitelist(env: Env, address: Address, whitelisted: bool)`: Persists the whitelisted status of an address. **Admin-only**, requires signature validation.
- `is_whitelisted(env: Env, address: Address) -> bool`: View function returning the whitelist status of an address.
- `set_whitelist_enforced(env: Env, enabled: bool)`: Toggles the whitelist enforcement flag. **Admin-only**, requires signature validation.
- `is_whitelist_enforced(env: Env) -> bool`: View function returning whether whitelist enforcement is enabled.

### 4. Payout Gating (Enforcement)
Modified payout flows in `program-escrow/src/lib.rs` to validate recipients:
- `single_payout()`: If `is_whitelist_enforced` is `true`, panics with `"Recipient not whitelisted"` if the recipient is not whitelisted.
- `batch_payout()`: If `is_whitelist_enforced` is `true`, iterates through all recipients and panics with `"Recipient not whitelisted"` if any recipient in the batch is not whitelisted.

Both functions clear the reentrancy guard (`reentrancy_guard::clear_entered(&env)`) on failure paths to prevent locking the contract state.

## Testing Status

Created a dedicated test suite under `program-escrow/src/test_whitelist.rs` containing 11 tests verifying the following scenarios:
1. `test_set_and_unset_whitelist`: Checks that the admin can successfully whitelist/unwhitelist addresses, and verifying the `WlChange` event.
2. `test_set_whitelist_requires_admin_auth`: Assures that setting the whitelist requires admin authorization.
3. `test_set_and_unset_whitelist_enforcement`: Tests changing the enforcement flag, and verifying the `WlEnfChg` event.
4. `test_set_whitelist_enforced_requires_admin_auth`: Assures that changing enforcement requires admin authorization.
5. `test_whitelist_enforcement_off_single_payout_succeeds`: Confirms that payouts to non-whitelisted recipients work as normal when enforcement is disabled (default).
6. `test_single_payout_with_enforcement_non_whitelisted_panics`: Confirms that a payout to a non-whitelisted recipient fails when enforcement is enabled.
7. `test_single_payout_with_enforcement_whitelisted_succeeds`: Confirms that a payout to a whitelisted recipient succeeds when enforcement is enabled.
8. `test_batch_payout_with_enforcement_non_whitelisted_panics`: Confirms that a batch payout fails if any recipient in the batch is not whitelisted.
9. `test_batch_payout_with_enforcement_whitelisted_succeeds`: Confirms that a batch payout succeeds if all recipients are whitelisted.
10. `test_batch_payout_enforcement_off_succeeds`: Confirms that batch payouts to non-whitelisted recipients succeed when enforcement is disabled.

## Security Considerations

- **Secure by Default**: Whitelist enforcement is off by default (`unwrap_or(false)`), maintaining backward compatibility and avoiding lockouts of legitimate recipients during initial deployment or updates.
- **Admin-only Operations**: Mutation functions (`set_whitelist` and `set_whitelist_enforced`) enforce admin validation checks using `admin.require_auth()`.
- **Atomic Batch Checks**: Batch payout enforcement evaluates all recipients before processing any transfers. If any recipient fails, the entire transaction is rolled back.
