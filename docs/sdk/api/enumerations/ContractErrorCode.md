[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / ContractErrorCode

# Enumeration: ContractErrorCode

Defined in: [src/errors.ts:64](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L64)

Unified enum of every known contract error across all Grainlify contracts.

Naming convention:
  - Program-escrow (original SDK) ........... no prefix
  - Bounty-escrow ...........................  BOUNTY_*
  - Governance ..............................  GOV_*
  - Circuit-breaker / error-recovery .......  CIRCUIT_*

Program AMOUNT_BELOW_MIN and AMOUNT_ABOVE_MAX map to the on-chain errors
  Error::AmountBelowMinimum = 8
  Error::AmountAboveMaximum = 9
Bounty-specific min/max, circuit-breaker, and claim-expiry errors use
separate BOUNTY_* codes because their on-chain discriminants differ.

## Enumeration Members

### ALREADY\_INITIALIZED

> **ALREADY\_INITIALIZED**: `"ALREADY_INITIALIZED"`

Defined in: [src/errors.ts:70](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L70)

***

### AMOUNT\_ABOVE\_MAX

> **AMOUNT\_ABOVE\_MAX**: `"AMOUNT_ABOVE_MAX"`

Defined in: [src/errors.ts:75](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L75)

***

### AMOUNT\_BELOW\_MIN

> **AMOUNT\_BELOW\_MIN**: `"AMOUNT_BELOW_MIN"`

Defined in: [src/errors.ts:74](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L74)

***

### BOUNTY\_ALREADY\_INITIALIZED

> **BOUNTY\_ALREADY\_INITIALIZED**: `"BOUNTY_ALREADY_INITIALIZED"`

Defined in: [src/errors.ts:79](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L79)

***

### BOUNTY\_AMOUNT\_ABOVE\_MAXIMUM

> **BOUNTY\_AMOUNT\_ABOVE\_MAXIMUM**: `"BOUNTY_AMOUNT_ABOVE_MAXIMUM"`

Defined in: [src/errors.ts:97](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L97)

***

### BOUNTY\_AMOUNT\_BELOW\_MINIMUM

> **BOUNTY\_AMOUNT\_BELOW\_MINIMUM**: `"BOUNTY_AMOUNT_BELOW_MINIMUM"`

Defined in: [src/errors.ts:96](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L96)

***

### BOUNTY\_BATCH\_SIZE\_MISMATCH

> **BOUNTY\_BATCH\_SIZE\_MISMATCH**: `"BOUNTY_BATCH_SIZE_MISMATCH"`

Defined in: [src/errors.ts:89](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L89)

***

### BOUNTY\_CIRCUIT\_BREAKER\_OPEN

> **BOUNTY\_CIRCUIT\_BREAKER\_OPEN**: `"BOUNTY_CIRCUIT_BREAKER_OPEN"`

Defined in: [src/errors.ts:98](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L98)

***

### BOUNTY\_CLAIM\_EXPIRED

> **BOUNTY\_CLAIM\_EXPIRED**: `"BOUNTY_CLAIM_EXPIRED"`

Defined in: [src/errors.ts:99](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L99)

***

### BOUNTY\_DEADLINE\_NOT\_PASSED

> **BOUNTY\_DEADLINE\_NOT\_PASSED**: `"BOUNTY_DEADLINE_NOT_PASSED"`

Defined in: [src/errors.ts:84](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L84)

***

### BOUNTY\_DUPLICATE\_ID

> **BOUNTY\_DUPLICATE\_ID**: `"BOUNTY_DUPLICATE_ID"`

Defined in: [src/errors.ts:90](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L90)

***

### BOUNTY\_EXISTS

> **BOUNTY\_EXISTS**: `"BOUNTY_EXISTS"`

Defined in: [src/errors.ts:81](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L81)

***

### BOUNTY\_FEE\_RECIPIENT\_NOT\_SET

> **BOUNTY\_FEE\_RECIPIENT\_NOT\_SET**: `"BOUNTY_FEE_RECIPIENT_NOT_SET"`

Defined in: [src/errors.ts:87](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L87)

***

### BOUNTY\_FUNDS\_NOT\_LOCKED

> **BOUNTY\_FUNDS\_NOT\_LOCKED**: `"BOUNTY_FUNDS_NOT_LOCKED"`

Defined in: [src/errors.ts:83](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L83)

***

### BOUNTY\_FUNDS\_PAUSED

> **BOUNTY\_FUNDS\_PAUSED**: `"BOUNTY_FUNDS_PAUSED"`

Defined in: [src/errors.ts:95](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L95)

***

### BOUNTY\_GOVERNANCE\_VERSION\_TOO\_LOW

> **BOUNTY\_GOVERNANCE\_VERSION\_TOO\_LOW**: `"BOUNTY_GOVERNANCE_VERSION_TOO_LOW"`

Defined in: [src/errors.ts:100](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L100)

***

### BOUNTY\_GOVERNANCE\_PROPOSAL\_NOT\_EXECUTABLE

> **BOUNTY\_GOVERNANCE\_PROPOSAL\_NOT\_EXECUTABLE**: `"BOUNTY_GOVERNANCE_PROPOSAL_NOT_EXECUTABLE"`

Defined in: [src/errors.ts:101](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L101)

***

### BOUNTY\_INSUFFICIENT\_FUNDS

> **BOUNTY\_INSUFFICIENT\_FUNDS**: `"BOUNTY_INSUFFICIENT_FUNDS"`

Defined in: [src/errors.ts:93](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L93)

***

### BOUNTY\_INVALID\_AMOUNT

> **BOUNTY\_INVALID\_AMOUNT**: `"BOUNTY_INVALID_AMOUNT"`

Defined in: [src/errors.ts:91](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L91)

***

### BOUNTY\_INVALID\_BATCH\_SIZE

> **BOUNTY\_INVALID\_BATCH\_SIZE**: `"BOUNTY_INVALID_BATCH_SIZE"`

Defined in: [src/errors.ts:88](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L88)

***

### BOUNTY\_INVALID\_DEADLINE

> **BOUNTY\_INVALID\_DEADLINE**: `"BOUNTY_INVALID_DEADLINE"`

Defined in: [src/errors.ts:92](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L92)

***

### BOUNTY\_INVALID\_FEE\_RATE

> **BOUNTY\_INVALID\_FEE\_RATE**: `"BOUNTY_INVALID_FEE_RATE"`

Defined in: [src/errors.ts:86](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L86)

***

### BOUNTY\_NOT\_FOUND

> **BOUNTY\_NOT\_FOUND**: `"BOUNTY_NOT_FOUND"`

Defined in: [src/errors.ts:82](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L82)

***

### BOUNTY\_NOT\_INITIALIZED

> **BOUNTY\_NOT\_INITIALIZED**: `"BOUNTY_NOT_INITIALIZED"`

Defined in: [src/errors.ts:80](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L80)

***

### BOUNTY\_REFUND\_NOT\_APPROVED

> **BOUNTY\_REFUND\_NOT\_APPROVED**: `"BOUNTY_REFUND_NOT_APPROVED"`

Defined in: [src/errors.ts:94](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L94)

***

### BOUNTY\_UNAUTHORIZED

> **BOUNTY\_UNAUTHORIZED**: `"BOUNTY_UNAUTHORIZED"`

Defined in: [src/errors.ts:85](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L85)

***

### CIRCUIT\_INSUFFICIENT\_BALANCE

> **CIRCUIT\_INSUFFICIENT\_BALANCE**: `"CIRCUIT_INSUFFICIENT_BALANCE"`

Defined in: [src/errors.ts:121](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L121)

***

### CIRCUIT\_OPEN

> **CIRCUIT\_OPEN**: `"CIRCUIT_OPEN"`

Defined in: [src/errors.ts:119](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L119)

***

### CIRCUIT\_TRANSFER\_FAILED

> **CIRCUIT\_TRANSFER\_FAILED**: `"CIRCUIT_TRANSFER_FAILED"`

Defined in: [src/errors.ts:120](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L120)

***

### EMPTY\_BATCH

> **EMPTY\_BATCH**: `"EMPTY_BATCH"`

Defined in: [src/errors.ts:71](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L71)

***

### GOV\_ALREADY\_VOTED

> **GOV\_ALREADY\_VOTED**: `"GOV_ALREADY_VOTED"`

Defined in: [src/errors.ts:113](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L113)

***

### GOV\_EXECUTION\_DELAY\_NOT\_MET

> **GOV\_EXECUTION\_DELAY\_NOT\_MET**: `"GOV_EXECUTION_DELAY_NOT_MET"`

Defined in: [src/errors.ts:115](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L115)

***

### GOV\_INSUFFICIENT\_STAKE

> **GOV\_INSUFFICIENT\_STAKE**: `"GOV_INSUFFICIENT_STAKE"`

Defined in: [src/errors.ts:106](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L106)

***

### GOV\_INVALID\_THRESHOLD

> **GOV\_INVALID\_THRESHOLD**: `"GOV_INVALID_THRESHOLD"`

Defined in: [src/errors.ts:104](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L104)

***

### GOV\_NOT\_INITIALIZED

> **GOV\_NOT\_INITIALIZED**: `"GOV_NOT_INITIALIZED"`

Defined in: [src/errors.ts:103](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L103)

***

### GOV\_PROPOSAL\_EXPIRED

> **GOV\_PROPOSAL\_EXPIRED**: `"GOV_PROPOSAL_EXPIRED"`

Defined in: [src/errors.ts:116](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L116)

***

### GOV\_PROPOSAL\_NOT\_ACTIVE

> **GOV\_PROPOSAL\_NOT\_ACTIVE**: `"GOV_PROPOSAL_NOT_ACTIVE"`

Defined in: [src/errors.ts:109](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L109)

***

### GOV\_PROPOSAL\_NOT\_APPROVED

> **GOV\_PROPOSAL\_NOT\_APPROVED**: `"GOV_PROPOSAL_NOT_APPROVED"`

Defined in: [src/errors.ts:114](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L114)

***

### GOV\_PROPOSAL\_NOT\_FOUND

> **GOV\_PROPOSAL\_NOT\_FOUND**: `"GOV_PROPOSAL_NOT_FOUND"`

Defined in: [src/errors.ts:108](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L108)

***

### GOV\_PROPOSALS\_NOT\_FOUND

> **GOV\_PROPOSALS\_NOT\_FOUND**: `"GOV_PROPOSALS_NOT_FOUND"`

Defined in: [src/errors.ts:107](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L107)

***

### GOV\_THRESHOLD\_TOO\_LOW

> **GOV\_THRESHOLD\_TOO\_LOW**: `"GOV_THRESHOLD_TOO_LOW"`

Defined in: [src/errors.ts:105](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L105)

***

### GOV\_VOTING\_ENDED

> **GOV\_VOTING\_ENDED**: `"GOV_VOTING_ENDED"`

Defined in: [src/errors.ts:111](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L111)

***

### GOV\_VOTING\_NOT\_STARTED

> **GOV\_VOTING\_NOT\_STARTED**: `"GOV_VOTING_NOT_STARTED"`

Defined in: [src/errors.ts:110](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L110)

***

### GOV\_VOTING\_STILL\_ACTIVE

> **GOV\_VOTING\_STILL\_ACTIVE**: `"GOV_VOTING_STILL_ACTIVE"`

Defined in: [src/errors.ts:112](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L112)

***

### GOVERNANCE\_VERSION\_TOO\_LOW

> **GOVERNANCE\_VERSION\_TOO\_LOW**: `"GOVERNANCE_VERSION_TOO_LOW"`

Defined in: [src/errors.ts:76](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L76)

***

### INSUFFICIENT\_BALANCE

> **INSUFFICIENT\_BALANCE**: `"INSUFFICIENT_BALANCE"`

Defined in: [src/errors.ts:68](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L68)

***

### INVALID\_AMOUNT

> **INVALID\_AMOUNT**: `"INVALID_AMOUNT"`

Defined in: [src/errors.ts:69](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L69)

***

### LENGTH\_MISMATCH

> **LENGTH\_MISMATCH**: `"LENGTH_MISMATCH"`

Defined in: [src/errors.ts:72](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L72)

***

### NOT\_INITIALIZED

> **NOT\_INITIALIZED**: `"NOT_INITIALIZED"`

Defined in: [src/errors.ts:66](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L66)

***

### OVERFLOW

> **OVERFLOW**: `"OVERFLOW"`

Defined in: [src/errors.ts:73](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L73)

***

### UNAUTHORIZED

> **UNAUTHORIZED**: `"UNAUTHORIZED"`

Defined in: [src/errors.ts:67](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L67)
