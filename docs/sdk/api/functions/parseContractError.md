[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / parseContractError

# Function: parseContractError()

> **parseContractError**(`error`): [`ContractError`](../classes/ContractError.md)

Defined in: [src/errors.ts:300](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/errors.ts#L300)

Parse a contract error from a Soroban response by matching the error message.
Falls back to a generic ContractError when no pattern matches.

Checks are ordered from most-specific to least-specific so that the more
descriptive min/max messages are matched before the generic INVALID_AMOUNT
fallback.

## Parameters

### error

`any`

## Returns

[`ContractError`](../classes/ContractError.md)
