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

The complete CLI command tutorial is generated with rustdoc and published at:

- <https://i3idev.github.io/gxx/gxx/index.html>

Generate the same page locally with:

```text
cargo doc --no-deps --document-private-items
```

The tutorials are written directly in the crate-level Rust documentation in
`src/main.rs`, so they appear on the main page at
`target/doc/gxx/index.html`.
## License

MIT