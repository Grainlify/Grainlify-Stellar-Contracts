# Event Versioning

## Rules

1. Legacy `v1` payloads may be unversioned.
2. Current `v2` payloads must include `version: 2`.
3. Newer payload versions (`v3+`) must preserve required fields used by indexers and SDK clients.
4. Parsers should:
   - default missing `version` to `1`
   - reject payloads missing required compatibility fields
   - ignore unknown additive fields

## Compatibility Guarantee

For event consumers in this repository:

- Backward compatibility: SDK parsing tests cover legacy/unversioned payloads.
- Forward compatibility: SDK parsing tests cover newer version tags with additive fields.
- Contract emission correctness: contract tests assert emitted payloads include `version: 2` tags on current emitters.
- Expiry/refund sweeps emit `BountyExpired` as a v2 payload before each `FundsRefunded` event so indexers can react without polling `query_expiring_bounties`.

## `SignerRot` Events (`grainlify-core/src/multisig.rs`)

`SignerRot` events (`"add"`, `"remove"`, `"thresh"`) are published as **raw positional tuples**
`(Address, Address, u32)` and carry no `version` field. They are therefore treated as **v1
(unversioned)** payloads under this policy. Indexers must parse them by tuple position, not
by key name.

Because these events do not carry a `version` field, any future breaking change to the tuple
shape (reordering, type change, or removal of a position) will require a **new topic symbol**
(e.g. `symbol_short!("SignerRot2")`) rather than an in-place `version` bump. Additive changes
(appending a new position at the end) are permitted without a topic change provided all
consumers are updated to ignore trailing unknown elements.

**Zero-events-on-revert:** All three `SignerRot` sub-types are published only after every
state mutation and guard check succeeds. A panicking transaction rolls back completely —
no partial `SignerRot` events are ever emitted.
