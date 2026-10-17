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

## Releases and downloads

Supported release targets:

| Platform | Architecture | Asset |
|---|---|---|
| Windows | x86_64 | `gxx-vX.Y.Z-windows-x86_64.exe` |
| Linux | x86_64 | `gxx-vX.Y.Z-linux-x86_64` |
| Debian/Ubuntu | amd64 | `gxx_X.Y.Z_amd64.deb` |
| macOS | Apple Silicon (aarch64) | `gxx-vX.Y.Z-macos-aarch64` |
| macOS | Intel (x86_64) | `gxx-vX.Y.Z-macos-x86_64` |

Each release also includes `SHA256SUMS`.

### Debian/Ubuntu

```bash
sudo apt install ./gxx_X.Y.Z_amd64.deb
sudo apt remove gxx
```

The Debian package targets Debian/Ubuntu amd64 and installs `/usr/bin/gxx`.
Other architectures require separate builds and packages.

### Verify checksums

```bash
sha256sum -c SHA256SUMS
```

### macOS
macOS binaries are unsigned in this project. macOS may warn when opening
downloaded binaries. Use the normal system controls to allow the
downloaded application; do not disable system security protections globally.

### Linux binary vs Debian package
The standalone Linux binary is a direct executable. The Debian package
installs the same binary into `/usr/bin/gxx` and records package metadata
for `apt`.

### Maintainer release process
Create and push a version tag:

```bash
git tag v0.2.1
git push origin v0.2.1
```

This triggers the `Release` workflow, which builds and publishes the
release assets. Manual workflow runs (`workflow_dispatch`) build and
upload artifacts without publishing a GitHub Release.

Runtime notes:
- The Linux build targets Ubuntu latest and may require a recent glibc
  (2.38+) plus `libstdc++6` / `libgcc-s1`.
- macOS binaries are architecture-specific, not universal binaries.
## License

MIT