# gxx

bilingual (English / Persian) word and grammar notebook CLI with a multi-voice Text-to-Speech subsystem.

`gxx` is a local, offline-first tool. Data lives in SQLite databases under `~/.gxx/db/` by default, or inside a project-local `.gxx/` directory when using the `--vault` option (similar to Obsidian vaults). TTS uses Piper voices discovered under `model/<lang>/<voice-quality>/`.

## Quick Start

```text
# 1. Initialize gxx data directory
gxx init

# 2. Create a database
gxx db new demo

# 3. Add a word (Persian translation auto-detected)
gxx add book کتاب -p n -l A1

# 4. Show the entry
gxx show book

# 5. Speak it
gxx say book

# 6. Read a file aloud
gxx paragraph --auto "hello سلام"
gxx read article.txt --auto --save-progress
```

## Documentation

Full documentation lives in [`docs/`](./docs/):

| Page | What's inside |
|------|---------------|
| [`docs/index.md`](docs/index.md) | Documentation home, navigation, Quick Start |
| [`docs/installation.md`](docs/installation.md) | Build from source, verify installation |
| [`docs/concepts.md`](docs/concepts.md) | Vocabulary entries, databases, active database, vaults |
| [`docs/database.md`](docs/database.md) | Database management commands, vault usage, `.gxx/` storage |
| [`docs/vocabulary.md`](docs/vocabulary.md) | `add`, `show`, `find`, `list`, `del`, fields, sections |
| [`docs/examples-and-notes.md`](docs/examples-and-notes.md) | Examples, notes, translations, definition field |
| [`docs/word-relations.md`](docs/word-relations.md) | Synonyms, antonyms, senses, collocations, word families |
| [`docs/grammar-style-form.md`](docs/grammar-style-form.md) | Grammar / style / form entries: add, list, show, del, fields, errors |
| [`docs/tts.md`](docs/tts.md) | `say`, `paragraph`, `read`, synthesis vs playback, mixed language |
| [`docs/models.md`](docs/models.md) | Piper model discovery, selection, MODEL_CARD, `gxx model` |
| [`docs/global-options.md`](docs/global-options.md) | `--db`, `--vault/-v`, `--json`, `--quiet/-q`, `--no-color` |
| [`docs/tutorials.md`](docs/tutorials.md) | End-to-end workflows: vocabulary notebook, grammar notebook, TTS, vault, shell scripts |
| [`docs/troubleshooting.md`](docs/troubleshooting.md) | Real error cases, diagnostics, fixes |
| [`docs/command-reference.md`](docs/command-reference.md) | Complete command and option index |
| [`docs/faq.md`](docs/faq.md) | Common user questions |

## License

MIT