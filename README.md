# gxx

Bilingual (English / Persian) word and grammar notebook CLI.

Ported from Python to Rust with identical behaviour.

## Installation

```bash
cargo build --release
```

The binary will be at `target/release/gxx`.

Run with `./target/release/gxx` or install to PATH:

```bash
cargo install --path .
```

## Quick Start

```bash
# 1. Initialize config directory (~/.gxx)
gxx init

# 2. Create a database
gxx db new demo

# 3. Add a word (English + Persian translations)
gxx add book کتاب -p n -l A1

# 4. Look it up
gxx book
```

Output:
```
================================================
book  (n)  [A1]
================================================

TRANSLATIONS
 fa : کتاب

NOTES : 

CREATED: 2026-10-10  UPDATED: 2026-10-10
================================================
```

## Usage

### Setup

```bash
gxx init                          # create ~/.gxx and config
gxx db new demo                   # create database "demo"
gxx db list                       # list databases (* = active)
gxx db use demo                   # set active database
gxx db info                       # show counts for active database
gxx db del demo --yes             # delete database (confirm with name)
```

### Words

```bash
# Add word with translations, POS, level
gxx add book کتاب -p n -l A1
gxx add run دویدن -p v -l A2
gxx add fast سریع -p adj -l A2

# Direct lookup (same as: gxx show book)
gxx book

# Show specific sections
gxx show book tr                  # translations only
gxx show book ex                  # examples only
gxx show book syn ant             # synonyms + antonyms

# Find (search in current DB)
gxx find کتاب                     # Persian query
gxx find book                     # English query
gxx find fast --all               # search ALL databases

# List with filters
gxx list                          # all words
gxx list --pos n                  # nouns only
gxx list --lvl A1                 # level A1 only
gxx list --lang en                # English-origin words only
gxx list --lang fa                # Persian-origin words only
```

Output example (`gxx list --pos n`):
```
book               n          A1   کتاب
house              n          A1   خانه
-- 2 words
```

### Translations, Synonyms, Antonyms

```bash
# Add/remove translations (+ add, - remove)
gxx tr book +volume +textbook
gxx tr book -سریع

# Add/remove synonyms
gxx syn book +volume +textbook

# Add/remove antonyms
gxx ant book +paperback +childrens

# List current
gxx syn book
gxx ant book
```

### Examples

```bash
# Add example (language auto-detected)
gxx ex book "I opened the book." "من کتاب را گشودم."

# Set one side of example N
gxx ex book 1 "I closed the book."

# Remove example N
gxx ex book -1

# List examples
gxx show book ex
```

Output:
```
EXAMPLES (1)
 1> I opened the book.
    = من کتاب را گشودم.
```

### Word Metadata

```bash
# Set / get fields
gxx pos book n
gxx pos book                        # -> n
gxx lvl book A1
gxx pron book bʊk
gxx def book "a set of pages"
gxx note book "review flashcards daily"
```

### Senses (Polysemy)

```bash
gxx sense book add "a set of pages for reading"
gxx sense book add "a volume in a series" --note "countable"
gxx sense book                      # list senses
gxx sense book show 1               # show sense #1
gxx sense book del 1                # delete sense #1
```

### Collocations

```bash
gxx coll book add "open a book"
gxx coll book add "close the book"
gxx coll book                       # list
gxx coll book del 1                 # delete #1
```

### Word Family

```bash
gxx family book add textbook -p n
gxx family book add booklet -p n
gxx family book                     # list
gxx family book del textbook        # delete
```

### Text-to-Speech

```bash
gxx say book                        # speak the word
gxx say book --example 1            # speak example #1
gxx say book --repeat 3 --slow      # repeat 3x at 0.7x speed
gxx say book --very-slow            # 0.5x speed
gxx say book --soft                 # lower volume
gxx say book --phonemes "dʒˈʌmps"   # manual IPA
```

> **Note**: Requires Piper TTS model. On first run it downloads a voice.

### Delete Word

```bash
gxx del book
```

### Grammar / Style / Form

Same commands work for `gram`, `style`, `form`:

```bash
gxx gram add past_tense --cat verb --lvl A2 --rule "v + ed"
gxx gram rule past_tense "add -ed to regular verbs"
gxx gram desc past_tense "regular past tense formation"
gxx gram list                       # list all grammar entries
gxx gram list past                  # filter by prefix
gxx gram show past_tense            # full entry
gxx gram ex past_tense "She walked home." "او به خانه رفت."
gxx gram err past_tense "walked"    # add error
gxx gram err past_tense -1          # remove error #1
gxx gram del past_tense
```

Output (`gxx gram show past_tense`):
```
================================================
past tense  [gram]  [en]  [A2]  (verb)
================================================

RULE : add -ed to regular verbs

DESC : regular past tense formation

EXAMPLES (1)
 1> She walked home.
    = او به خانه رفت.

NOTES : 

CREATED: 2026-10-10  UPDATED: 2026-10-10
================================================
```

### Scripted Commands (`.gxx` files)

```bash
gxx load commands.gxx
```

File format (one command per line, `#` = comment, leading `gxx` optional):

```gxx
# seed notebook
add cat گربه -p n -l A1
add dog سگ -p n -l A1
tr cat میانبر
syn cat kitten
ant cat dog
ex cat "The cat sat." "گربه نشست."
```

### Options

```bash
--db <name>                # use another database for this command only
gxx --db other list         # list words in "other" DB
```

### JSON Output

```bash
gxx show book --json
gxx list --pos n --json
gxx find کتاب --json
```

## Features

- **Bilingual**: English and Persian (Farsi) support
- **Persian normalization**: Arabic ي→ی, ك→ک, ZWNJ (U+200C) removed for search/lookup
- **Cross-compatible**: Databases created by Python version work with Rust version and vice versa
- **Plain text output**: No colors, no emoji, ASCII-only layout
- **Multiple databases**: `~/.gxx/db/<name>.db`, active DB in `~/.gxx/config`
- **Underscores as spaces**: Multi-word items fit in one argument (`open_a_book` → "open a book")

## Data Model

- SQLite database with single `entries` table
- JSON stored in `data` column (unknown fields preserved via serde flatten)
- Entry ID: `<kind>:<lowercased title>` (not normalized)
- Searchable `text` column is normalized on save

## Environment

- `GXX_HOME`: Override default `~/.gxx` directory

## License

MIT