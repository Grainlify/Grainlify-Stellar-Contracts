[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / parseContractError

# Function: parseContractError()

> **parseContractError**(`error`): [`ContractError`](../classes/ContractError.md)

Defined in: [src/errors.ts:300](https://github.com/Grainlify/Grainlify-Stellar-Contracts/blob/6f3c3b734f3968e8d999ddcb9f7b52e0fd13ce9f/sdk/src/errors.ts#L300)

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
