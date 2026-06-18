# Grainlify Governance System

## Overview

The Grainlify governance system enables decentralized decision-making for contract upgrades through proposals, authenticated voting, quorum checks, and approval thresholds. Governance configuration explicitly selects either one-person-one-vote or token-weighted voting.

## Key Parameters

- **Voting Period:** Duration during which votes can be cast.
- **Execution Delay:** Time-lock period after a proposal is approved before it can be executed.
- **Quorum:** Minimum percentage, in basis points, of the scheme-specific total voting power that must participate.
- **Approval Threshold:** Minimum percentage, in basis points, of non-abstaining voting power that must vote `For`.
- **Minimum Proposal Stake:** Minimum balance of the configured governance token required to create a proposal.
- **Governance Token:** Soroban token address used for token-weighted voting and proposal-stake checks.
- **Token Total Voting Power:** Total token voting power used as the denominator for token-weighted quorum. This should match the selected snapshot or stake-lock set.

## Voting Power

### OnePersonOneVote

`OnePersonOneVote` assigns every authenticated voter a constant voting power of `1`. The contract prevents the same address from voting more than once on the same proposal.

Because the contract does not maintain an on-chain voter registry, quorum for this scheme is calculated against `one_person_total_voters` from `GovernanceConfig`. Deployments must keep this value aligned with the eligible electorate.

### TokenWeighted

`TokenWeighted` derives each vote's `voting_power` by reading the voter's balance from the configured governance token contract at vote time:

```text
voting_power = governance_token.balance(voter)
```

The contract rejects votes with zero voting power. Token-weighted quorum is calculated against `GovernanceConfig::token_total_voting_power`, which should represent the total governance-token power eligible at the selected snapshot or stake-lock point.

## Snapshot And Balance Semantics

The standard Soroban token interface exposes current balances, not historical balances. `GovernanceConfig::snapshot_ledger` records the ledger selected by governance policy for a snapshot or stake-lock process, but the contract cannot independently query historical token balances from a normal token contract.

Production token-weighted governance should use one of these mitigations:

- Lock voting stake for the full voting window before proposals can be voted on.
- Use a governance token wrapper that exposes snapshot balances for the configured snapshot ledger.
- Ensure token supply and transferable balances cannot be cheaply manipulated during the voting window.

Without one of these controls, a voter may temporarily acquire tokens, vote, and transfer them away before finalization. The contract mitigates zero-balance voting and uses the configured token address for all reads, but current-balance voting alone does not prevent flash-loan style power inflation.

## Governance Flow

1. **Proposal Creation**
   - The proposer must authorize the call.
   - If `min_proposal_stake > 0`, the proposer must hold at least that much of the configured governance token.
   - Voting starts immediately upon creation.

2. **Voting Period**
   - Eligible voters cast `For`, `Against`, or `Abstain`.
   - The contract derives voting power according to the configured voting scheme.
   - Each address can vote once per proposal.
   - Zero-power votes are rejected.

3. **Finalization**
   - After the voting period ends, anyone can call `finalize_proposal`.
   - Quorum is checked against the scheme-correct total voting power.
   - Approval threshold is checked against `For + Against` voting power, excluding abstentions.
   - If quorum is not met, the proposal is stored as `Rejected`.

4. **Execution**
   - Approved proposals enter the configured execution delay before upgrade execution.
   - Execution logic should preserve the existing time-lock and audit requirements.

## Security Features

- **Authenticated Voting:** `require_auth()` is called for voters and proposers.
- **Double-Voting Prevention:** Each address can vote only once per proposal.
- **Configured Token Reads:** Token-weighted power and stake checks use only `GovernanceConfig::governance_token`.
- **Zero-Power Rejection:** Accounts with no scheme-valid voting power cannot vote.
- **Minimum Stake Requirement:** Proposal creation can require governance-token ownership.
- **Quorum Enforcement:** Participation is checked before approval threshold math.

---

*Grainlify Governance - Empowering Decentralized Evolution*
