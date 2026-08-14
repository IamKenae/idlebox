<div align="center">

# IdleBox

**Say goodbye to Busy, embrace Idle.**

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org)
[![Size](https://img.shields.io/badge/size-~360KB-green.svg)](target/release/idlebox)

[🇨🇳 中文文档](README-zh.md)

</div>

---

## Introduction

**IdleBox** is an independent, lightweight, and visually polished BusyBox/POSIX-compatible toolbox written in pure Rust with zero external dependencies.

### Design Philosophy

> Say goodbye to Busy, embrace Idle.

BusyBox has powered embedded Linux for over two decades. IdleBox reimagines this classic concept in modern Rust—maintaining POSIX compatibility while pursuing a smaller footprint, stronger safety guarantees, and a more delightful terminal experience.

---

## Features

- **Zero Dependencies** — Pure Rust standard library, no third-party crates
- **Ultra-compact** — ~360KB release binary, ideal for embedded and container environments
- **POSIX Compatible** — Drop-in replacement for common Unix utilities
- **Modular Design** — Easily extend via the Applet mechanism
- **Symlink Support** — Invoke applets directly via symlinks
- **Beautiful Terminal Output** — Built-in ANSI color support for a delightful CLI experience

---

## Implemented Applets

| Applet | Description | Highlights |
|--------|-------------|------------|
| `echo` | Print text to standard output | Supports `-n` (no newline), `-e` (escape interpretation) |
| `relax` | IdleBox special: take a break and relax | A unique relaxation experience, embodying the "Idle" spirit |
| `cat` | Concatenate files and print to standard output | Supports `-n` line numbers, `-b` non-blank numbering, `-A` show invisibles, stdin pipe |
| `ls` | List directory contents | **ANSI colorized output**: dirs in blue, executables in green, archives in red, symlinks in cyan; supports `-l` long format, `-a` hidden files, `-h` human-readable sizes |

---

## Quick Start

### Build

```bash
# Debug build
cargo build

# Release build (optimized for size)
cargo build --release

# Check binary size
ls -lh target/release/idlebox
```

### Run

```bash
# Direct invocation
idlebox echo "Hello, IdleBox!"
idlebox cat -n README.md
idlebox ls --color=auto -lah

# Via symlink
cd target/release
ln -s idlebox echo
ln -s idlebox ls
./echo "Hello via symlink!"
./ls --color=auto
```

### Test

```bash
cargo test
```

---

## Adding New Applets

1. Create a new file in `src/applets/`
2. Implement the `Applet` trait
3. Export it in `src/applets/mod.rs`
4. Register it in `src/core/dispatcher.rs`

---

## Architecture Documentation

The detailed architecture documentation has been moved to a separate documentation repository to keep the main repository minimal and pure.

> 📖 **View Architecture Docs**: [IdleBox Docs](https://github.com/IamKenae/idlebox-docs)

---

## License

[Apache-2.0](LICENSE)

Copyright (c) IdleBox Contributors.
