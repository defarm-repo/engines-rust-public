# Repository Guidelines

## Project Structure & Module Organization
The core Rust engine lives in `src/`. `src/lib.rs` exposes domain modules like `adapters/`, `api/`, `events_engine.rs`, and persistence layers under `postgres_*`. Executables reside in `src/bin/` (`defarm-api`) and `src/main.rs` (`defarm-demo`). Integration tests are collected in `tests/`, while migration SQL and shell helpers sit in `migrations/`, `init-db.sh`, and `docker-compose.yml`. Reference specifications and operational runbooks are under `docs/` to keep architecture and adapter behavior aligned with code.

## Build, Test, and Development Commands
- `cargo check` — fast type-check to validate incremental edits.
- `cargo build` / `cargo build --release` — compile API binaries; release mode matches production builds.
- `cargo fmt --all` and `cargo clippy --all-targets -- -D warnings` — enforce formatting and lint gates.
- `cargo test` — run unit + integration suite; use `cargo test --test integration_tests` to focus on multi-service scenarios.
- `./test-events.sh`, `./test-circuit-workflow.sh` — smoke tests for adapters and circuit orchestration against local Postgres.

## Coding Style & Naming Conventions
Rustfmt (4-space indent, trailing commas) is the source of truth; run it before committing. Follow idiomatic Rust naming: modules and functions in `snake_case`, types and traits in `UpperCamelCase`, constants in `SCREAMING_SNAKE_CASE`. Group adapter-specific logic under `src/adapters/` and keep storage interfaces behind the traits in `src/storage.rs`; prefer `tracing` spans for structured logging.

## Testing Guidelines
Add focused unit tests alongside modules (e.g., `src/circuit_tokenization_tests.rs`) and broader scenarios under `tests/`. Integration specs should stand up async contexts with `#[tokio::test]` and reuse fixtures in `init-db.sh` to prep Postgres. Every PR should run `cargo test` and relevant `test-*.sh` scripts; document new edge cases in `docs/` if behavior changes.

## Commit & Pull Request Guidelines
History follows Conventional Commits (`feat:`, `fix:`, `docs:`). Keep messages imperative and scoped to a single concern. PRs must include: a concise summary, linked issue or doc reference, validation notes (`cargo test`, script outputs), and screenshots or API samples when modifying surface behavior.

## Environment & Configuration Tips
Use `docker-compose up postgres api` to mirror production services and load migrations. Keep secrets in `.env` files (not committed) to satisfy the environment variables consumed by `docker-compose` and the binaries. Update `deploy.sh` and `docs/` when changing adapter endpoints or Stellar credentials.
