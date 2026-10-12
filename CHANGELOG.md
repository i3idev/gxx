# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-10-12

### Added — TTS subsystem

- Multi-model TTS registry with `model/<lang>/<voice-quality>/` layout
- Recursive model discovery (depth 3) and stable IDs (hash by filename stem)
- Language guard: clear error when text language doesn't match the voice
- `strip_language_tags()` removes eSpeak `(en)` / `(fa)` markers from IPA
- `segment_by_language()` splits mixed text into per-language runs
- `--vowel-stretch N` adds extra `ː` marks after each long vowel (0–3)
- `--length-scale`, `--noise-scale`, `--noise-w` override Piper inference params
- Cache key includes `onnx_path`, `vowel_stretch`, `noise_w` to avoid collisions

### Added — new commands

- `gxx paragraph <text>`: free-text TTS, no DB lookup, no word cache
  - `--voice <CODE|auto>` override (language code or voice name)
  - `--auto` routes each segment to a matching model; unknown languages
    are skipped with a warning
  - `--file <PATH>` read text from a file
  - `--phonemes <IPA>` skip eSpeak and use raw IPA
  - `--out <PATH>` write WAV instead of playing
  - `--sentence-split` + `--sentence-pause-ms`
  - `--list-voices` shows voices with MODEL_CARD defaults
- `gxx read <file>`: narrate a file paragraph by paragraph
  - `--auto`, `--voice`, `--start N`, `--out PATH`
  - `--save-progress` / `--resume` / `--clear-progress` bookmarks
  - `Ctrl+C` stops after the current paragraph
- `gxx model list` / `use` / `current` / `info` already existed; now backed
  by the new registry

### Added — MODEL_CARD presets

- MODEL_CARD may declare `* VowelStretch`, `* LengthScale`, `* NoiseScale`,
  `* NoiseW`
- Applied automatically unless the CLI overrides them
- `fa_IR/amir-medium` ships with `VowelStretch=2`, `LengthScale=1.10`,
  `NoiseScale=0.80`

### Changed

- `gxx say` now goes through the new multi-model loader
- legacy `-r` / `--read` GUI flags removed
- paragraph splitting prefers `\n\n`, falls back to single `\n`
- model IDs are now stable across directory moves

### Tests

- 78 tests total: 62 unit, 12 CLI, 4 reading (2 ignored: require models)

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

