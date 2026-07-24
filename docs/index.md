# Grainlify Stellar Contracts - Documentation Index


Welcome to the Grainlify Stellar Contracts documentation. This index provides a centralized entry point for all project documentation, organized by topic.


## Architecture and Overview


| Document | Description |
|----------|-------------|
| [Architecture](ARCHITECTURE.md) | Smart contract architecture for escrow and program escrow contracts |
| [Versions](VERSIONS.md) | Contract versioning policy and compatibility matrix |
| [Contributing](CONTRIBUTING.md) | Contribution guidelines and build artifact hygiene |
| [Storage and TTL Model](STORAGE_AND_TTL.md) | Soroban storage classes, TTL handling, archival risk, and extension guidance |


## Contracts


### Bounty Escrow


| Document | Description |
|----------|-------------|
| [Bounty Escrow README](../bounty_escrow/README.md) | Overview of the bounty escrow contract |
| [Security](bounty_escrow/SECURITY.md) | Security model, threat analysis, and mitigations |
| [Circuit Breaker](bounty_escrow/CIRCUIT_BREAKER.md) | Circuit breaker mechanism documentation |
| [Implementation Checklist](bounty_escrow/IMPLEMENTATION_CHECKLIST.md) | Implementation task tracking |
| [Analytics Documentation](bounty_escrow/ANALYTICS_DOCUMENTATION.md) | Analytics views and query functions |
| [Analytics Implementation Summary](bounty_escrow/ANALYTICS_IMPLEMENTATION_SUMMARY.md) | Analytics module implementation details |
| [Feature Completion Report](bounty_escrow/FEATURE_COMPLETION_REPORT.md) | Feature delivery status report |
| [Auto Refund Tests](bounty_escrow/contracts/escrow/AUTO_REFUND_TESTS.md) | Auto-refund test documentation |
| [CI Checks Summary](bounty_escrow/contracts/escrow/CI_CHECKS_SUMMARY.md) | CI pipeline checks overview |


### Program Escrow


| Document | Description |
|----------|-------------|
| [Program Escrow README](../program-escrow/README.md) | Overview of the program escrow contract |
| [Reentrancy Guard](program-escrow/REENTRANCY_GUARD_DOCUMENTATION.md) | Reentrancy guard mechanism documentation |
| [Analytics Events](program-escrow/ANALYTICS_EVENTS.md) | Event structures and analytics integration |
| [Implementation Summary](program-escrow/IMPLEMENTATION_SUMMARY.md) | Query functions implementation summary |


### Core


| Document | Description |
|----------|-------------|
| [Event Schema](EVENT_SCHEMA.md) | Comprehensive event schema reference for all contracts |
| [Event Versioning](EVENT_VERSIONING.md) | Event payload versioning rules and compatibility |

## Governance

| Document | Description |
|----------|-------------|
| [Governance Integration](GOVERNANCE_INTEGRATION.md) | Governance integration across contracts |

## Queries and SDK

| Document | Description |
|----------|-------------|
| [Query Documentation](QUERY_DOCUMENTATION.md) | Query function reference and usage |
| [Query Quick Reference](QUERY_QUICK_REFERENCE.md) | Quick reference for query functions |
| [SDK README](../sdk/README.md) | TypeScript SDK overview and usage |
| [SDK API Reference](sdk/api/README.md) | Generated TypeDoc reference for SDK classes, methods, and exported types |
| [SDK Error Mapping](sdk/ERROR_MAPPING.md) | SDK error types and mapping |

## Security

| Document | Description |
|----------|-------------|
| [Bounty Escrow Security](bounty_escrow/SECURITY.md) | Security model for the bounty escrow contract |
| [Reentrancy Guard](program-escrow/REENTRANCY_GUARD_DOCUMENTATION.md) | Reentrancy guard for program escrow |
| [Circuit Breaker](bounty_escrow/CIRCUIT_BREAKER.md) | Circuit breaker safety mechanism |

## Testing and Delivery

| Document | Description |
|----------|-------------|
| [Admin Tests Summary](ADMIN_TESTS_SUMMARY.md) | Admin function test coverage summary |
| [Implementation Summary](IMPLEMENTATION_SUMMARY.md) | Escrow history query implementation summary |
| [Feature Delivery Summary](FEATURE_DELIVERY_SUMMARY.md) | Bounty escrow analytics feature delivery |

## Soroban

| Document | Description |
|----------|-------------|
| [Soroban README](soroban/README.md) | Soroban project structure and setup |
| [Program Escrow Duplication Audit](soroban/PROGRAM_ESCROW_DUPLICATION_AUDIT.md) | Audit of `soroban/contracts/program-escrow` vs the production `program-escrow` crate, including TTL-fix parity findings |

## Scripts

| Document | Description |
|----------|-------------|
| [Scripts README](../scripts/README.md) | Build, deployment, and utility scripts |
| [Deployment Runbook](DEPLOYMENT_RUNBOOK.md) | End-to-end deploy, verify, upgrade, rollback, registry, and mainnet safety procedures |

---

## Navigation Guide

- **New contributors**: Start with [Architecture](ARCHITECTURE.md) and [Contributing](CONTRIBUTING.md)
- **Contract developers**: Review the contract-specific sections under [Contracts](#contracts)
- **Frontend/SDK developers**: See [Queries and SDK](#queries-and-sdk)
- **Security reviewers**: Focus on the [Security](#security) section
- **Event consumers**: Refer to [Events](#events) for schema and versioning
