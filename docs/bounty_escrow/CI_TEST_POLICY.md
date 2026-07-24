# Bounty Escrow CI Test Policy

The `bounty-escrow-tests` GitHub Actions job runs the full `bounty_escrow` test suite with:

```bash
cargo test
```

CI must not exclude the analytics tests or the event-version tag coverage. Those tests guard the aggregate accounting views and the versioned event schema consumed by downstream indexers, so skipping them can hide regressions in externally visible contract behavior.

As of this policy update, the previously skipped analytics tests and `test_events_emit_v2_version_tags_for_all_bounty_emitters` pass deterministically in the full local suite. If a future test becomes flaky, fix the root cause or document a narrowly scoped special handling path instead of adding broad `--skip` filters to CI.