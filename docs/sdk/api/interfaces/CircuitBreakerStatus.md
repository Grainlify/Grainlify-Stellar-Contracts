[**@grainlify/contracts-sdk**](../README.md)

***

[@grainlify/contracts-sdk](../README.md) / CircuitBreakerStatus

# Interface: CircuitBreakerStatus

Defined in: [src/bounty-escrow-client.ts:199](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L199)

Current status snapshot of the circuit breaker.

## Properties

### failure\_count

> **failure\_count**: `number`

Defined in: [src/bounty-escrow-client.ts:203](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L203)

Number of consecutive failures in closed state.

***

### failure\_threshold

> **failure\_threshold**: `number`

Defined in: [src/bounty-escrow-client.ts:211](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L211)

The error count threshold to open the circuit.

***

### last\_failure\_timestamp

> **last\_failure\_timestamp**: `bigint`

Defined in: [src/bounty-escrow-client.ts:207](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L207)

Timestamp of the last recorded failure.

***

### opened\_at

> **opened\_at**: `bigint`

Defined in: [src/bounty-escrow-client.ts:209](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L209)

Timestamp of when the circuit was opened.

***

### state

> **state**: [`CircuitState`](../type-aliases/CircuitState.md)

Defined in: [src/bounty-escrow-client.ts:201](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L201)

The state of the circuit breaker.

***

### success\_count

> **success\_count**: `number`

Defined in: [src/bounty-escrow-client.ts:205](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L205)

Number of consecutive successes in half-open state.

***

### success\_threshold

> **success\_threshold**: `number`

Defined in: [src/bounty-escrow-client.ts:213](https://github.com/mxrtins04/Grainlify-Stellar-Contracts/blob/34042aba00c5f308f7440b49b36a077874089c25/sdk/src/bounty-escrow-client.ts#L213)

The success count threshold to close the circuit.
