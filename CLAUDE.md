# grimoire

Before writing or reviewing code in a bounded context, invoke `/personal:rust-ddd` (variant: small).

## Layout

- One crate at the repo root, one bounded context: `src/{domain,application,infrastructure,presentation}`
  beside `src/kernel` and `src/bootstrap.rs` (configuration, migrations, wiring). The binary is still
  named `grimoire-server`.
- The architecture test is `cargo test --test architecture`; CI-equivalent checks are
  `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`.
- `web/` is the React UI; it talks to the API under `/api`. Errors come back as `{code, message}`.

## Readings of the standard for this repo

- **Actors.** The web UI's user is the only client actor, so every scenario has a `client/` level
  (`application/<scenario>/client/`). `organization/job/` holds the startup reconciliation.
  Handler modules are `presentation/web/<scenario>_client.rs`.
- **Database.** The single-context crate keeps one `migrations/` sequence and unprefixed table names.
  `0001`…`0010` predate the restructure and must never be edited (sqlx checks their checksums);
  add new migrations with `sqlx migrate add` (timestamp versions).
- **Ids** are `kernel::Id<E>` over UUIDs; new ones are v7, existing rows keep their v4 ids.

## Deliberate deviations

- Commands without an Output answer `204 No Content`, not a bare body: the web client relies on it.
- `ExecuteMovesUseCase`, `UndoMoveBatchUseCase` and `StartSourceRefetchUseCase` take `self: Arc<Self>`:
  they run detached in `tokio::spawn`, so a dropped request never stops a move between a file and
  its record.
- The re-fetch progress is in-process state shared by its two use cases (`Arc<Mutex<SourceRefetch>>`),
  not a repository: it only lives as long as the process.
- `KeywordRuleRepositoryError::KeywordTaken` is a second variant beside `Backend`: uniqueness is
  enforced by a database index, and the conflict is the user's to see.
