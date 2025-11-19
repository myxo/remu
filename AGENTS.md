# Repository Guidelines

## Project Structure & Module Organization
Core scheduling, parsing, and persistence live in `src/` (`engine.rs`, `command.rs`, `database.rs`, `state.rs`, `sql_query.rs`), while `prop_test.rs` stores property-style regressions behind the `mock-time` feature. The old `remu.py` file is kept only as historical reference and is not part of the runtime. Local artifacts (`database.db`, `token.id`) support development only and should remain untracked.

## Build, Test, and Development Commands
`cargo check` keeps edit/compile cycles fast, while `cargo build --release` readies deployable artifacts. `cargo run --bin remu` starts the worker and now covers end-to-end behavior by itself. Validate functionality with `cargo test --all-targets` (add `--features mock-time` when logic depends on the clock) and keep formatting/lints clean via `cargo fmt --all` plus `cargo clippy --all-targets --all-features -D warnings`.

## Coding Style & Naming Conventions
Default to `rustfmt` (4-space indents, trailing commas, grouped imports). Modules stay `snake_case`, public types use `CamelCase`, constants use screaming snake (`MOMENT_DAY_REGEX`), and errors propagate with `anyhow::Result` + `?`. Favor explicit `use` lists and log through `log`/`env_logger`.

## Testing Guidelines
Add unit tests next to the logic they cover (`#[cfg(test)] mod tests` in `command.rs`, `database.rs`, etc.). Complex invariants belong in `src/prop_test.rs` using the `chaos_theory` helpers—name cases after the behavior they guard (`should_parse_day_boundary`). Use `mock-time` whenever wall-clock math matters so `cargo test --features mock-time` stays deterministic, and ship both a regression test and a persistence/Telegram flow for every feature.

## Commit & Pull Request Guidelines
Existing history favors short, imperative subjects (`random updates`, `get rid of ...`); keep commits under ~60 characters, present tense, and scoped to one concern. Pull requests must summarize the change, list the commands you ran (build, test, bridge), flag schema/config migrations, and link related issues. Attach screenshots or Telegram transcripts for UX adjustments.

## Security & Configuration Notes
Document secrets but never check in `token.id`. When touching SQLite schema, update the SQL in `sql_query.rs`, describe the migration steps, and explain how to upgrade an existing `database.db`.
