#!/usr/bin/env bash
set -euo pipefail

printf "=== Verify DataKey variants and storage access patterns ===\n"

for file in \
  bounty_escrow/contracts/escrow/src/lib.rs \
  program-escrow/src/lib.rs \
  grainlify-core/src/lib.rs; do
  printf "\n--- %s ---\n" "$file"
  awk '/pub enum DataKey|enum DataKey/,/}/' "$file" | sed '1d;$d'
done

printf "\n=== Storage access patterns ===\n"
grep -REn "storage\(\)\\.(persistent|instance)\(|extend_ttl\(" \
  bounty_escrow/contracts/escrow/src/lib.rs \
  program-escrow/src/lib.rs \
  grainlify-core/src/lib.rs
