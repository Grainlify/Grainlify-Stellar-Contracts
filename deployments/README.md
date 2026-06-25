# Deployment Registry

This directory contains machine-readable deployment registry JSON files for tracking active smart contract deployments on different networks.

## Schema

Each registry file `deployments/<network>.json` contains a JSON object with:
- `deployments`: An array of deployment records.
- `metadata`: A metadata object containing `created` timestamp and schema `version`.

### Deployment Record Fields

| Field | Type | Description |
|-------|------|-------------|
| `contract_name` | String | Name of the contract (e.g., `escrow`) |
| `contract_id` | String | Stellar contract ID starting with `C` (56 characters) |
| `wasm_hash` | String | SHA-256 hash of the compiled WASM binary (64 hex characters) |
| `soroban_sdk` | String | Soroban SDK version used to build the contract |
| `deployer` | String | Deployer identity name or address |
| `timestamp` | String | ISO-8601 timestamp of deployment |
| `network` | String | Stellar network name |
| `status` | String | Status of the contract (`deployed`) |

### Example Registry File

```json
{
  "deployments": [
    {
      "contract_name": "escrow",
      "contract_id": "CESCROW1234567890123456789012345678901234567890123456789",
      "wasm_hash": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
      "soroban_sdk": "21.0.0",
      "deployer": "mock-deployer",
      "timestamp": "2026-06-25T12:00:00Z",
      "network": "testnet",
      "deployed_at": "2026-06-25T12:00:00Z",
      "status": "deployed"
    }
  ],
  "metadata": {
    "created": "2026-06-25T12:00:00Z",
    "version": "1.0"
  }
}
```

## Management Scripts

The following scripts integrate directly with the registry:
- **`deploy.sh`**: Registers a new deployment record under `deployments/<network>.json` upon success.
- **`upgrade_contract.sh` / `upgrade.sh`**: Adds an updated record to the network registry.
- **`verify-deployment.sh`**: Automatically resolves contract names to IDs and expected hashes from the registry for drift detection.
- **`rollback.sh`**: Retrieves the previous version (second-to-last by date) from the registry to execute code rollback and records the new rolled-back state.
