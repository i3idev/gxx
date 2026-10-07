# gxx

Bilingual (English / Persian) word and grammar notebook CLI.

Ported from Python to Rust with identical behaviour.

## Installation

```bash
cargo build --release
```

The binary will be at `target/release/gxx`.

## Usage

```bash
# Setup
gxx init
gxx db new <name>
gxx db list
gxx db use <name>
gxx db info
gxx db del <name> [--yes]

# Words
gxx add <word> [translations...] [-p pos] [-l level]
gxx <word>                 # same as: gxx show <word> [tr|syn|ant|ex]
gxx find <text> [--all]
gxx list [--pos x] [--lvl x] [--lang en|fa]
gxx tr  <word> [lang] +x -y
gxx syn|ant <word> +x -y
gxx ex  <word> ["text"]            # add example (language detected)
gxx ex  <word> N "text"            # set one side of example N
gxx ex  <word> -N                  # remove example N
gxx pos|lvl|pron|def|note <word> [value]
gxx del <word>

# Grammar / Style / Form
gxx gram add <name> [--cat c] [--lvl l] [--rule r] [--desc d]
gxx gram rule|desc|cat|lvl|note <name> [value]
gxx gram ex  <name> ...
gxx gram err <name> ["text"] [-N]
gxx gram list [prefix]
gxx gram del <name>
gxx gram show <name>

# Files
gxx load <file.gxx>        # run a file of commands (one per line, # = comment)

# Options
--db <name>                # use another database for this command only
```

## Features

- **Bilingual**: English and Persian (Farsi) support
- **Persian normalization**: Arabic ي→ی, ك→ک, ZWNJ (U+200C) removed for search/lookup
- **Cross-compatible**: Databases created by Python version work with Rust version and vice versa
- **Plain text output**: No colors, no emoji, ASCII-only layout
- **Multiple databases**: `~/.gxx/db/<name>.db`, active DB in `~/.gxx/config`
- **Underscores as spaces**: Multi-word items fit in one argument

## Data Model

- SQLite database with single `entries` table
- JSON stored in `data` column (unknown fields preserved via serde flatten)
- Entry ID: `<kind>:<lowercased title>` (not normalized)
- Searchable `text` column is normalized on save

## Environment

- `GXX_HOME`: Override default `~/.gxx` directory

## License

MIT# gxx
