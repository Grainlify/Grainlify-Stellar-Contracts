# Deployment and upgrade runbook

This runbook documents the operational flow for deploying, initializing, verifying, upgrading, and rolling back Grainlify Soroban contracts. It is based on the current scripts in `scripts/` and the deployment registry files under `deployments/`.

## Operator safety model

- Never commit private keys, funded secret keys, or machine-local identity files.
- Prefer stored Stellar/Soroban CLI identities such as `grainlify-deployer` or `grainlify-mainnet-deployer` instead of passing secrets on the command line.
- Run `--dry-run` first for every new deployment or upgrade command.
- Treat mainnet as irreversible. Mainnet deploys and upgrades require explicit confirmation in the scripts, and rollbacks require a second confirmation unless `--force` is used.
- Rollback changes contract code only. It does not revert contract state or repair incompatible storage migrations.

## Prerequisites

Install the tooling required by the scripts:

```bash
stellar --version
jq --version
cargo --version
rustup target add wasm32-unknown-unknown
```

The shared helper in `scripts/utils/common.sh` selects `stellar` first and falls back to `soroban`. It also requires `jq` for registry writes.

Create and fund the deployer identity before running live commands:

```bash
stellar keys generate --global grainlify-deployer
stellar keys fund grainlify-deployer --network testnet
stellar keys address grainlify-deployer
```

For mainnet, create a separate identity and fund it through the normal mainnet treasury process. Do not reuse the testnet identity.

## Configuration files

The deployment scripts load `scripts/config/<network>.env` by default unless `--config` is supplied.

| File | Network | Purpose |
| --- | --- | --- |
| `scripts/config/local.env` | local | Uses `http://localhost:8000/rpc`, local network passphrase, `grainlify-local-deployer`, `deployments/local.json`, shorter timeouts, and confirmation disabled. |
| `scripts/config/testnet.env` | testnet | Uses SDF testnet RPC and Friendbot, `grainlify-deployer`, `deployments/testnet.json`, confirmation enabled, and state backup enabled before upgrades. |
| `scripts/config/mainnet.env` | mainnet | Uses Stellar mainnet RPC, no Friendbot, `grainlify-mainnet-deployer`, `deployments/mainnet.json`, longer timeouts, confirmation enabled, and state backup enabled. |

Important environment variables:

| Variable | Used by | Meaning |
| --- | --- | --- |
| `SOROBAN_RPC_URL` | deploy, verify, upgrade, rollback | RPC endpoint for the selected network. |
| `SOROBAN_NETWORK_PASSPHRASE` | config and CLI context | Passphrase for local, testnet, or mainnet. |
| `SOROBAN_NETWORK` | all scripts | Network name passed to the CLI. |
| `FRIENDBOT_URL` | local/testnet setup | Funding endpoint for non-mainnet identities. |
| `DEPLOYER_IDENTITY` | deploy, verify, upgrade, rollback | Local CLI identity used for signing or read calls. |
| `WASM_DIR` | operator convention | Directory where release WASM files are expected. |
| `DEFAULT_CONTRACTS` | operator convention | Comma-separated contracts to deploy in scripted batches. |
| `DEPLOYMENT_LOG` | deploy | Registry path written by `append_to_registry`. |
| `CLI_TIMEOUT` | all scripts | Timeout for each CLI call. |
| `RETRY_ATTEMPTS` | deploy, upgrade | Number of retries for network-sensitive operations. |
| `RETRY_DELAY` | deploy, upgrade | Delay between retries in seconds. |
| `REQUIRE_CONFIRMATION` | confirmation helper | Controls non-mainnet confirmation behavior. Mainnet still has hard script-level gates. |
| `DRY_RUN` | deploy, upgrade, rollback | Simulates operations without sending transactions. |
| `BACKUP_STATE` | operator policy | Indicates whether state should be backed up before upgrades. |

Command-line flags take precedence over config values for network, identity/source, config file path, dry-run, verbose logging, and rollback force mode.

## Build WASM

Build release WASM before deploying:

```bash
cargo build --target wasm32-unknown-unknown --release
```

Expected release artifacts are referenced from:

```bash
soroban/target/wasm32-unknown-unknown/release
```

Before a live deployment, confirm the exact `.wasm` path and record the commit SHA used to build it.

## Deploy and initialize

Preferred script:

```bash
./scripts/deploy.sh <wasm_file> [options] -- [init args]
```

Common options:

| Option | Meaning |
| --- | --- |
| `-n, --network <local|testnet|mainnet|futurenet>` | Target network. Defaults to `testnet`. |
| `-c, --config <file>` | Override the default config file. |
| `-i, --identity <name>` | Override `DEPLOYER_IDENTITY`. |
| `-N, --name <contract_name>` | Registry name. Defaults to the WASM filename without `.wasm`. |
| `--init` | Call `init` after deployment. |
| `--init-args '<args>'` | Legacy init argument string. Prefer passthrough args after `--`. |
| `--dry-run` | Print the planned install/deploy/init steps without sending transactions. |
| `-v, --verbose` | Enable debug logs. |

Recommended dry run:

```bash
./scripts/deploy.sh soroban/target/wasm32-unknown-unknown/release/escrow.wasm \
  -n testnet \
  -N escrow \
  --dry-run
```

Deploy and initialize with passthrough init arguments:

```bash
./scripts/deploy.sh soroban/target/wasm32-unknown-unknown/release/escrow.wasm \
  -n testnet \
  -N escrow \
  -- \
  --admin GADMIN... \
  --fee 100
```

What the script does:

1. Validates the WASM path and network.
2. Loads `scripts/config/<network>.env` unless `--config` is supplied.
3. Checks CLI dependencies, `jq`, network connectivity, and deployer identity.
4. On mainnet, prints a mainnet warning and requires confirmation.
5. Installs the WASM and captures the `wasm_hash`.
6. Deploys the contract and captures the `contract_id`.
7. Optionally invokes `init`.
8. Appends the deployment to the configured registry.

If init fails after deploy, the contract id is still printed and the deployment may already be in the registry. Record the failure, verify the deployed contract manually, and either run the correct `init` invocation manually or decide whether a fresh deploy is required.

## Verify a deployment

Preferred script:

```bash
./scripts/verify-deployment.sh <contract_id> [options]
```

Common options:

| Option | Meaning |
| --- | --- |
| `-n, --network <network>` | Target network. Defaults to `testnet`. |
| `-s, --source <identity>` | Identity used for read calls. |
| `-c, --config <file>` | Override config file. |
| `-f, --function <name>` | Verification function. Defaults to `get_version`. |
| `--check-admin` | Also attempt to read admin ownership. |
| `--expected-admin <address>` | Compare the admin getter result with an expected address. |
| `--expected-wasm <path>` | Calculate SHA-256 for a local WASM artifact and compare it with the deployed WASM hash. |
| `--expected-wasm-hash <hash>` | Compare the deployed WASM hash with a known expected hash. |
| `--smoke-functions <list>` | Comma-separated read-only functions to call. Defaults to `get_version,get_admin,get_pause_flags`. |
| `--skip-smoke` | Skip the default read-only smoke checks. |
| `--json` | Emit machine-readable JSON. |
| `-v, --verbose` | Enable debug logs. |

Examples:

```bash
./scripts/verify-deployment.sh CCONTRACT... -n testnet
./scripts/verify-deployment.sh CCONTRACT... -n mainnet --json
./scripts/verify-deployment.sh CCONTRACT... --check-admin --expected-admin GADMIN...
./scripts/verify-deployment.sh CCONTRACT... --expected-wasm target/escrow.wasm
./scripts/verify-deployment.sh CCONTRACT... --expected-wasm-hash 7a8b9c0d...
```

By default, verification now performs the primary `--function` check and read-only
smoke calls for `get_version`, `get_admin`, and `get_pause_flags`. Use
`--smoke-functions` for contracts with a different view surface, or `--skip-smoke`
when verifying a minimal contract that does not expose those getters. When a WASM
artifact or expected hash is provided, the script reads the deployed hash with
`stellar contract info hash --contract-id` and fails the verification if the
hash cannot be read or does not match.

Exit codes:

| Code | Meaning |
| --- | --- |
| `0` | Contract is healthy. |
| `1` | Contract is unresponsive or verification failed. |
| `2` | Invalid arguments or configuration error. |

## Upgrade

Preferred script:

```bash
./scripts/upgrade.sh <contract_id> <new_wasm_path> [options]
```

Common options:

| Option | Meaning |
| --- | --- |
| `-n, --network <network>` | Target network. Defaults to `testnet`. |
| `-s, --source <identity>` | Source identity authorized as contract admin. |
| `-c, --config <file>` | Override config file. |
| `--skip-verify` | Skip the post-upgrade responsiveness check. |
| `--dry-run` | Print planned install and upgrade calls without sending transactions. |
| `-v, --verbose` | Enable debug logs. |

Recommended dry run:

```bash
./scripts/upgrade.sh CCONTRACT... soroban/target/wasm32-unknown-unknown/release/escrow.wasm \
  -n testnet \
  --dry-run
```

Live upgrade:

```bash
./scripts/upgrade.sh CCONTRACT... soroban/target/wasm32-unknown-unknown/release/escrow.wasm \
  -n testnet \
  -s grainlify-deployer
```

What the script does:

1. Validates the contract id, WASM path, and network.
2. Loads config and verifies the source identity.
3. On mainnet, prints a mainnet warning and requires confirmation.
4. Installs the new WASM and captures `new_wasm_hash`.
5. Calls `upgrade --new_wasm_hash <hash>` on the contract.
6. Unless `--skip-verify` is set, calls `get_version` to check responsiveness.
7. Appends an upgrade entry to `deployments/upgrades.json`.

The older `scripts/upgrade_contract.sh` is a thin helper that uploads a WASM with `soroban contract upload` and calls the same `upgrade --new_wasm_hash` entrypoint. Prefer `scripts/upgrade.sh` for reviewed operations because it includes config loading, dry-run mode, mainnet confirmation, verification, and registry logging.

## Rollback

Preferred script:

```bash
./scripts/rollback.sh <contract_id> <previous_wasm_hash> [options]
```

Common options:

| Option | Meaning |
| --- | --- |
| `-n, --network <network>` | Target network. Defaults to `testnet`. |
| `-s, --source <identity>` | Source identity authorized as contract admin. |
| `-c, --config <file>` | Override config file. |
| `--force` | Skip confirmation prompts. Dangerous, especially on mainnet. |
| `--dry-run` | Print the planned rollback call without sending a transaction. |
| `-v, --verbose` | Enable debug logs. |

Find the rollback hash:

```bash
jq -r '.upgrades[-1].old_wasm_hash' deployments/upgrades.json
jq -r '.deployments[] | select(.contract_name == "escrow") | .wasm_hash' deployments/testnet.json
```

Dry run:

```bash
./scripts/rollback.sh CCONTRACT... <previous_wasm_hash> -n testnet --dry-run
```

Live rollback:

```bash
./scripts/rollback.sh CCONTRACT... <previous_wasm_hash> -n testnet -s grainlify-deployer
```

What the script does:

1. Validates the contract id, rollback WASM hash, and network.
2. Loads config and verifies the source identity.
3. Prints a critical warning that state is not reverted.
4. Requires confirmation, and requires a second confirmation on mainnet.
5. Calls `upgrade --new_wasm_hash <previous_wasm_hash>`.
6. Appends a rollback entry to `deployments/rollbacks.json`.
7. Prints a post-rollback checklist.

After rollback, always run:

```bash
./scripts/verify-deployment.sh CCONTRACT... -n <network>
```

Then test the critical write paths and review whether storage/data migration is needed.

## Deployment registry

The `deployments/` directory is the operational registry. The committed `.gitkeep` documents the expected live files:

- `deployments/local.json`
- `deployments/testnet.json`
- `deployments/mainnet.json`
- `deployments/upgrades.json`
- `deployments/rollbacks.json`

Deployment registries are created by `append_to_registry` in `scripts/utils/common.sh`:

```json
{
  "deployments": [
    {
      "contract_id": "CCONTRACT...",
      "wasm_hash": "abcdef...",
      "contract_name": "escrow",
      "network": "testnet",
      "deployer": "grainlify-deployer",
      "deployed_at": "2026-06-18T00:00:00Z",
      "status": "deployed"
    }
  ],
  "metadata": {
    "created": "2026-06-18T00:00:00Z",
    "version": "1.0"
  }
}
```

Upgrade records are stored in `deployments/upgrades.json`:

```json
{
  "upgrades": [
    {
      "contract_id": "CCONTRACT...",
      "old_wasm_hash": "oldhash...",
      "new_wasm_hash": "newhash...",
      "contract_name": "escrow",
      "wasm_file_hash": "filehash...",
      "network": "testnet",
      "upgraded_by": "grainlify-deployer",
      "upgraded_at": "2026-06-18T00:00:00Z",
      "status": "completed"
    }
  ],
  "metadata": {
    "created": "2026-06-18T00:00:00Z",
    "version": "1.0"
  }
}
```

Rollback records are stored in `deployments/rollbacks.json`:

```json
{
  "rollbacks": [
    {
      "contract_id": "CCONTRACT...",
      "target_wasm_hash": "oldhash...",
      "network": "testnet",
      "rolled_back_by": "grainlify-deployer",
      "reason": "manual rollback",
      "executed_at": "2026-06-18T00:00:00Z",
      "status": "completed",
      "data_migration_required": "REVIEW NEEDED"
    }
  ],
  "metadata": {
    "created": "2026-06-18T00:00:00Z",
    "version": "1.0"
  }
}
```

Registry consumers:

- `deploy.sh` writes deployment records to the configured `DEPLOYMENT_LOG`.
- `upgrade.sh` writes upgrade records to `deployments/upgrades.json`.
- `rollback.sh` expects the operator to provide `previous_wasm_hash`, usually from `deployments/upgrades.json` or a network registry file, and writes rollback records to `deployments/rollbacks.json`.
- `verify-deployment.sh` does not write the registry; it reads config and checks live contract responsiveness.

Review registry files before committing them. They do not contain secret keys by design, but contract ids, deployment timing, and operational metadata may still be sensitive for unreleased deployments.

## Mainnet pre-flight checklist

Before any mainnet deploy, upgrade, or rollback:

- Confirm the source commit and release tag.
- Confirm the WASM was built from the reviewed commit.
- Run testnet deployment or upgrade first.
- Run `--dry-run` with the exact mainnet command.
- Confirm `scripts/config/mainnet.env` points to `https://soroban.stellar.org`.
- Confirm `SOROBAN_NETWORK_PASSPHRASE` is `Public Global Stellar Network ; September 2015`.
- Confirm `DEPLOYER_IDENTITY` is the mainnet identity and has enough XLM.
- Confirm admin addresses, token addresses, oracle addresses, and fee values.
- Back up existing registry files.
- Capture the old WASM hash before upgrade.
- Confirm rollback hash availability and whether the previous WASM is installed on network.
- Assign one operator to run the command and one reviewer to read the prompt aloud.

The scripts include mainnet gates:

- `deploy.sh` prints a mainnet deployment warning and asks `Deploy to MAINNET? This action cannot be undone.`
- `upgrade.sh` prints a mainnet upgrade warning and asks `Proceed with MAINNET upgrade?`
- `rollback.sh` prints a rollback data warning and asks for confirmation; for mainnet it also asks `FINAL CONFIRMATION: Execute MAINNET rollback?`

Do not use `--force` on mainnet unless an incident commander explicitly approves skipping prompts.

## Troubleshooting

| Symptom | Likely cause | Response |
| --- | --- | --- |
| `Neither 'stellar' nor 'soroban' CLI found` | CLI is not installed or not on `PATH`. | Install the Stellar CLI or Soroban CLI and reopen the shell. |
| `'jq' is required but not installed` | Registry JSON helper is missing. | Install `jq` before deploy, upgrade, rollback, or JSON verification. |
| `Identity not found` | `DEPLOYER_IDENTITY` or `--source` does not exist locally. | Create/import the identity, or pass the correct identity name. |
| WASM file validation fails | Path is wrong or build did not produce release WASM. | Rebuild with `cargo build --target wasm32-unknown-unknown --release` and pass the exact `.wasm` path. |
| Network connectivity fails | RPC URL, passphrase, or network name is wrong. | Check the selected config file and retry against the correct network. |
| Deploy succeeds but init fails | Init args are wrong or the contract does not support the expected init call. | Keep the contract id, review args, run manual init if safe, and document the failure in the registry notes. |
| Upgrade invocation fails | Source is not admin, new WASM hash is invalid, or contract lacks `upgrade`. | Confirm admin identity, install hash, and contract upgrade interface before retrying. |
| Post-upgrade verification warns about `get_version` | The contract may not expose `get_version`. | Run `verify-deployment.sh -f <known_read_function>` or perform manual read checks. |
| Rollback fails because hash is not installed | Previous WASM hash is not available on network. | Install the old WASM first, then rerun rollback using the installed hash. |
| Rollback succeeds but behavior is broken | Contract state may be incompatible with old code. | Stop writes, run verification, inspect storage expectations, and plan manual data migration. |

## End-to-end operator checklist

1. Build: `cargo build --target wasm32-unknown-unknown --release`
2. Dry-run deploy: `./scripts/deploy.sh <wasm> -n <network> --dry-run`
3. Deploy and optionally init: `./scripts/deploy.sh <wasm> -n <network> -N <name> -- [init args]`
4. Verify: `./scripts/verify-deployment.sh <contract_id> -n <network>`
5. Save registry files and deployment output.
6. Dry-run upgrade: `./scripts/upgrade.sh <contract_id> <new_wasm> -n <network> --dry-run`
7. Upgrade: `./scripts/upgrade.sh <contract_id> <new_wasm> -n <network> -s <admin_identity>`
8. Verify again with `verify-deployment.sh`.
9. If rollback is needed, find `old_wasm_hash`, run `rollback.sh --dry-run`, then execute rollback only after confirming state compatibility risk.
