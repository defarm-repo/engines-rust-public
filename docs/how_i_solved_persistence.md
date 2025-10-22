# How I Solved the Persistence Issue

- Added an explicit `wait_for_connection` guard before circuit persistence so queue workers don’t fire while the pool is still `None`, preventing in-memory-only writes (`src/postgres_persistence.rs:504`).
- The guard logs a debug hint with the offending circuit id and exits early when the database is still offline, allowing the retry loop to back off instead of failing silently (`src/postgres_persistence.rs:505`).
- Tightened the same connection guard for items and events to include entity identifiers in the debug log so retries carry enough context when PostgreSQL is still warming up (`src/postgres_persistence.rs:1210`, `src/postgres_persistence.rs:1301`).
