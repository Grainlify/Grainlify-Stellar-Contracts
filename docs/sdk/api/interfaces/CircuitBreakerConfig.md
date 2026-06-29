[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / CircuitBreakerConfig

# Interface: CircuitBreakerConfig

Defined in: [src/bounty-escrow-client.ts:186](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L186)

Configuration for the circuit breaker.

## Properties

### failure\_threshold

> **failure\_threshold**: `number`

Defined in: [src/bounty-escrow-client.ts:188](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L188)

Count of consecutive errors required to open the circuit.

***

### max\_error\_log

> **max\_error\_log**: `number`

Defined in: [src/bounty-escrow-client.ts:192](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L192)

Maximum number of records in the error log.

***

### success\_threshold

> **success\_threshold**: `number`

Defined in: [src/bounty-escrow-client.ts:190](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L190)

Count of consecutive successes required to close the circuit in half-open state.
