# Governance Integration Documentation

## Overview

This document describes the integration of grainlify-core governance state into bounty and program escrow contracts for upgrade and configuration control.

## Architecture

The governance integration follows a modular design that allows escrow contracts to respect governance decisions without tight coupling:

```
┌─────────────────────────────────────────────────────────────┐
│                  Governance Integration                      │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────────┐         ┌──────────────────┐         │
│  │  Grainlify-Core  │         │  Escrow Contract │         │
│  │   Governance     │◄────────│  (Bounty/Program)│         │
│  └──────────────────┘         └──────────────────┘         │
│         │                              │                     │
│         │ Version Check                │                     │
│         │ Upgrade Approval             │                     │
│         │                              │                     │
│         ▼                              ▼                     │
│  ┌──────────────────┐         ┌──────────────────┐         │
│  │  Proposal System │         │  Admin Operations│         │
│  │  - Create        │         │  - Pause/Unpause │         │
│  │  - Vote          │         │  - Fee Config    │         │
│  │  - Execute       │         │  - Rate Limits   │         │
│  └──────────────────┘         └──────────────────┘         │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

## Key Components

### 1. Governance Hooks

The escrow contracts honor the following governance hooks:

- **Version Check**: Ensures the governance contract meets minimum version requirements
- **Upgrade Approvals**: Validates that upgrades are approved through governance
- **Configuration Control**: Respects governance decisions for critical configuration changes

### 2. Storage Keys

```rust
const GOVERNANCE_CONTRACT: Symbol = symbol_short!("GOV_ADDR");
const MIN_GOV_VERSION: Symbol = symbol_short!("MIN_VER");
```

### 3. Core Functions

#### Setting Governance Contract

```rust
pub fn set_governance_contract(env: Env, governance_addr: Address)
```

Admin-only function to configure the governance contract address.

#### Setting Minimum Version

```rust
pub fn set_min_governance_version(env: Env, min_version: u32)
```

Admin-only function to set the minimum required governance version.

#### Version Check

```rust
fn check_governance_version(env: &Env) -> bool
```

Internal function that validates the governance contract version meets requirements.

## Integration Points

### Admin Operations Protected by Governance

The following admin operations now check governance requirements:

#### Program Escrow
- `set_paused()` - Pause/unpause operations
- `update_rate_limit_config()` - Rate limit configuration

#### Bounty Escrow
- `set_paused()` - Pause/unpause operations
- `update_fee_config()` - Fee configuration
- `update_multisig_config()` - Multisig configuration

### Governance Check Flow

```rust
fn check_governance_requirements(env: &Env) {
    if !governance_integration::check_governance_version(env) {
        panic!("Governance version requirement not met");
    }
}
```

This check is called at the beginning of protected admin operations.

## Usage Examples

### 1. Setting Up Governance

```rust
// Initialize escrow contract
let escrow_client = ProgramEscrowContractClient::new(&env, &contract_id);
escrow_client.set_admin(&admin);

// Deploy and configure governance
let gov_client = GrainlifyContractClient::new(&env, &gov_contract_id);
gov_client.init_governance(&admin, &governance_config);

// Link escrow to governance
escrow_client.set_governance_contract(&gov_contract_id);
escrow_client.set_min_governance_version(&2);
```

### 2. Admin Operations with Governance

```rust
// This will check governance version before executing
escrow_client.set_paused(&Some(true), &None, &None);

// If governance version < min_version, operation will panic
```

### 3. Upgrading with Governance

```rust
// Create upgrade proposal in governance
let proposal_id = gov_client.create_proposal(
    &proposer,
    &new_wasm_hash,
    &symbol_short!("upgrade")
);

// Vote on proposal
gov_client.cast_vote(&voter1, &proposal_id, &VoteType::For);
gov_client.cast_vote(&voter2, &proposal_id, &VoteType::For);

// Finalize and execute
gov_client.finalize_proposal(&proposal_id);

// Upgrade is now approved and can be executed
```

## Backward Compatibility

The governance integration is designed to be backward compatible:

1. **Optional Governance**: Contracts work without governance configured
2. **Gradual Migration**: Governance can be added to existing deployments
3. **Version Flexibility**: Minimum version can be adjusted as needed

### Migration Path

For existing deployments:

```rust
// Step 1: Deploy governance contract
let gov_contract_id = deploy_governance(&env);

// Step 2: Link to escrow (no disruption)
escrow_client.set_governance_contract(&gov_contract_id);

// Step 3: Set minimum version (optional)
escrow_client.set_min_governance_version(&2);

// Step 4: Existing operations continue to work
```

## Security Considerations

### 1. Admin Authorization

All governance configuration functions require admin authorization:

```rust
let admin: Address = env.storage().instance().get(&DataKey::Admin).unwrap();
admin.require_auth();
```

### 2. Version Validation

The system validates governance version before critical operations:

```rust
if version < min_version {
    panic!("Governance version requirement not met");
}
```

### 3. Upgrade Safety

Upgrades must be approved through governance when configured:

```rust
pub fn check_upgrade_approval(env: &Env, wasm_hash: &BytesN<32>) -> bool {
    if let Some(gov_addr) = get_governance_contract(env) {
        // Check governance approval
        check_governance_version(env)
    } else {
        // No governance, allow upgrade
        true
    }
}
```

## Testing

### Test Coverage

The integration includes comprehensive tests:

1. **Basic Configuration**
   - Setting governance contract
   - Setting minimum version
   - Retrieving configuration

2. **Version Checks**
   - Successful version validation
   - Failed version validation
   - Operations without governance

3. **Integration Tests**
   - Full lifecycle with governance
   - Admin operations with governance
   - Upgrade scenarios

### Running Tests

```bash
# Test program-escrow governance integration
cd program-escrow
cargo test test_governance_integration

# Test bounty-escrow governance integration
cd bounty_escrow/contracts/escrow
cargo test test_governance_integration

# Test grainlify-core governance
cd grainlify-core
cargo test
```

## Best Practices

### 1. Governance Setup

- Deploy governance contract first
- Configure with appropriate voting parameters
- Test on testnet before mainnet

### 2. Version Management

- Start with minimum version 0 for gradual rollout
- Increment version for breaking changes
- Document version requirements

### 3. Upgrade Process

- Create proposal with clear description
- Allow sufficient voting period
- Test upgrade on testnet first
- Monitor execution

### 4. Emergency Procedures

- Keep admin key secure for emergency actions
- Document rollback procedures
- Maintain previous WASM hashes

## Troubleshooting

### Common Issues

1. **"Governance version requirement not met"**
   - Check governance contract version
   - Verify minimum version setting
   - Ensure governance contract is deployed

2. **"Not initialized"**
   - Initialize contract with admin
   - Set governance contract address
   - Configure minimum version

3. **Authorization failures**
   - Verify admin address
   - Check auth requirements
   - Ensure proper signatures

## Future Enhancements

Potential improvements for future versions:

1. **Proposal-Specific Approvals**: Check specific proposal approvals for upgrades
2. **Multi-Contract Governance**: Support governance across multiple contracts
3. **Timelock Integration**: Add timelock delays for critical operations
4. **Veto Mechanism**: Allow governance to veto admin actions
5. **Delegation**: Support vote delegation in governance

## References

- [Grainlify Core Governance](../grainlify-core/GOVERNANCE.md)
- [Program Escrow README](../program-escrow/README.md)
- [Bounty Escrow Security](../bounty_escrow/SECURITY.md)
- [Soroban Documentation](https://soroban.stellar.org/docs)

## Changelog

### Version 1.0.0 (Current)
- Initial governance integration
- Version checking
- Admin operation protection
- Comprehensive test coverage
- Backward compatibility support
