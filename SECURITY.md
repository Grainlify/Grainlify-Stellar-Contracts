# Security Policy

## Scope

This repository contains fund-moving Soroban smart contracts deployed on the Stellar network:

- **bounty_escrow** — see [`docs/bounty_escrow`](./docs/bounty_escrow) for contract-specific documentation.
- **program-escrow** — see [`docs/program-escrow`](./docs/program-escrow) for contract-specific documentation.
- **grainlify-core** — see [`docs/grainlify-core`](./docs/grainlify-core) for contract-specific documentation.
- **sdk** — see [`docs/sdk`](./docs/sdk) for integration/client-library documentation.
- **soroban** — see [`docs/soroban`](./docs/soroban) for contract-specific documentation.

Vulnerabilities in any contract deployed from this repository are in scope, including logic errors, access-control bypasses, arithmetic/overflow issues, and reentrancy or cross-contract callback vulnerabilities.

<!-- MAINTAINER ACTION NEEDED: deployments/ currently contains no recorded addresses (only .gitkeep). Populate this directory with live contract addresses and networks (mainnet/testnet) as they are deployed. -->
Current deployment addresses and networks (mainnet/testnet) for each contract will be listed under [`deployments/`](./deployments) as they go live. This directory is not yet populated — until a contract has a recorded deployment there, treat reports against it as pre-production/best-effort rather than a confirmed live-fund vulnerability.

## Reporting a Vulnerability

<!-- MAINTAINER ACTION NEEDED: replace with a real, monitored contact (e.g. security@<domain>, a private HackerOne/Immunefi program link, or a maintainer's PGP-verified email). Do not use a public GitHub issue for vulnerability reports. -->
**Contact:** `security@PLACEHOLDER-please-fill-in.example` *(placeholder — needs a maintainer to confirm a real, monitored channel)*

To report a suspected vulnerability:

1. **Do not open a public GitHub issue** describing the vulnerability.
2. Email the contact above with:
   - A description of the vulnerability and its potential impact.
   - Steps to reproduce, or a proof-of-concept if available.
   - The contract(s) and deployment(s) affected (see `deployments/` above).
3. If you'd like to encrypt your report, request a PGP key from the contact above before sending sensitive details.

## Response Timeline

<!-- MAINTAINER ACTION NEEDED: confirm these SLAs match the team's actual capacity. -->
| Stage | Target Timeline |
|---|---|
| Initial acknowledgment of report | Within 48 hours |
| Preliminary assessment / severity triage | Within 5 business days |
| Fix or mitigation plan communicated to reporter | Within 30 days (severity-dependent) |
| Public disclosure (coordinated with reporter) | After a fix is deployed, or by mutual agreement |

We ask researchers to give us reasonable time to investigate and remediate an issue before any public disclosure.

## Out of Scope

- Vulnerabilities in third-party dependencies not modified by this repository (report these upstream).
- Contracts explicitly marked as deprecated or unused test scaffolding, unless they remain deployed with real funds.
- Issues requiring access to compromised maintainer credentials or infrastructure outside these contracts.

## Recognition

<!-- MAINTAINER ACTION NEEDED: state whether a bug bounty or acknowledgment program exists for this repo. -->
We appreciate responsible disclosure. Recognition or reward details, if any, will be confirmed by a maintainer.
