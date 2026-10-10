# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-10-10

### Added
- Initial Rust implementation of `gxx`
- SQLite-backed bilingual word and grammar notebook
- Database management commands: `init`, `db new`, `db list`, `db use`, `db info`, `db del`
- Word management commands: add, show, find, list, delete, translations, synonyms, antonyms, examples, and metadata updates
- Grammar/style/form management commands
- Scripted command loading via `gxx load <file.gxx>`
- Persian normalization for improved search and lookup behavior
- Cross-version compatibility with the prior Python implementation
- `--db <name>` override for per-command database selection

### Features
- Bilingual English / Persian vocabulary support
- Searchable local notebook stored in SQLite
- Plain-text ASCII output and no external UI dependency
- Support for multiple databases under `~/.gxx/db`
- Environment override via `GXX_HOME`

### Notes
- This is the first public release of the Rust port.
- The project remains a CLI tool focused on local, offline study workflows.

