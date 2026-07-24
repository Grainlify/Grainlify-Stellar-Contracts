# Continuous integration

The main GitHub Actions workflow (`.github/workflows/ci.yml`) runs on pushes to the primary development branches and on every pull request.

## TypeScript SDK job

`TypeScript SDK - Build & Test` protects the first-class SDK under `sdk/` in parallel with the Rust/Soroban jobs.

The job:

1. checks out the repository;
2. installs Node.js 20 with npm caching keyed by `sdk/package-lock.json`;
3. runs `npm ci` in `sdk/` so dependency resolution is lockfile-backed and reproducible;
4. runs `npm run build` to fail on TypeScript compile/type errors; and
5. runs `npm test -- --runInBand` to fail on Jest regressions.

Keep `sdk/package-lock.json` committed whenever SDK dependencies change; otherwise `npm ci` intentionally fails instead of silently resolving a different dependency graph.