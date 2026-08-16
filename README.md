<div align="center">

# IdleBox

**Say goodbye to Busy, embrace Idle.**

[![License](https://img.shields.io/badge/license-Apache--2.0-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-2021-orange.svg)](https://www.rust-lang.org)
[![Linux x86_64 ELF size](https://img.shields.io/badge/size-~640_KiB-green.svg)](https://github.com/IamKenae/idlebox/actions/workflows/size.yml)

[🇨🇳 中文文档](README-zh.md)

</div>

---

## Introduction

**IdleBox** is an independent, lightweight, and visually polished multi-call toolbox inspired by BusyBox, written in Rust with a deliberately small pure-Rust dependency footprint and no bundled C libraries.

### Design Philosophy

> Say goodbye to Busy, embrace Idle.

BusyBox has powered embedded Linux for over two decades. IdleBox reimagines its multi-call binary concept in modern Rust and progressively improves compatibility with common POSIX, BusyBox, and GNU workflows.

The current stage focuses on making IdleBox itself better first: preserving flexibility, a small footprint, low overhead, and high performance while improving the project structure, core functionality, and user experience. Broader and deeper compatibility follows incrementally. This is a current engineering priority, not a permanent limit on the project's long-term direction.

### Current Development Principles

1. **Protect the lightweight foundation** — Prefer a single binary, a minimal pure-Rust dependency set, modularity, and low runtime overhead
2. **Optimize the project first** — Improve correctness, consistency, core functionality, cross-platform behavior, and maintainability
3. **Expand compatibility progressively** — Cover common workflows first, then add more POSIX, BusyBox, and GNU behavior
4. **Make evidence-based trade-offs** — Evaluate features and abstractions using binary size, startup time, throughput, and test results

---

## Features

- **Pure-Rust Compression** — DEFLATE and Gzip use `flate2` with the `miniz_oxide` Rust backend; no zlib or other C compression library
- **Size-conscious** — Release settings prioritize a compact binary; actual size varies by target and toolchain
- **Progressive Compatibility** — Covers common Unix/POSIX workflows first and incrementally expands BusyBox/GNU behavior
- **Cross-Platform** — Supports Linux, macOS, and Windows
- **Modular Design** — Easily extend via the Applet mechanism
- **Symlink Support** — Invoke applets directly via symlinks
- **Beautiful Terminal Output** — Built-in ANSI color support for a delightful CLI experience

---

## Platform Support

| Platform | Status | Notes |
|----------|--------|-------|
| Linux | Full | All 58 applets supported |
| macOS | Full | All 58 applets supported |
| Windows | Partial | 41+ applets fully supported; Unix-only applets (chmod, chown, chgrp, id, su) gracefully degrade |

---

## Implemented Applets

| Applet | Description | Highlights |
|--------|-------------|------------|
| `echo` | Print text to standard output | Supports `-n` (no newline), streams arguments without assembling a second full output string |
| `printf` | Format and print arguments | Reusable formats, backslash escapes, `%s`/`%b`/`%c`, integer and floating-point conversions, width and precision |
| `true` / `false` | Return a fixed exit status | Script-friendly success and failure primitives with no output |
| `env` | Run a command in a modified environment | `-i` clean environment, `-u` removal, temporary `NAME=VALUE` assignments, command execution |
| `printenv` | Print environment variables | Prints the whole environment or selected variables, with optional NUL separators |
| `pwd` | Print the current working directory | `-L` logical path and `-P` physical path resolution |
| `basename` | Strip directories and suffixes from names | Optional suffix removal, multiple operands with `-a`/`-s`, NUL-separated output |
| `dirname` | Strip the last component from names | Multiple operands and NUL-separated output |
| `cat` | Concatenate files and print to standard output | Supports `-n` line numbers, `-b` non-blank numbering, `-A` show invisibles, stdin pipe |
| `ls` | List directory contents | **ANSI colorized output**: dirs in blue, executables in green, archives in red, symlinks in cyan; supports `-l` long format, `-a` hidden files, `-h` human-readable sizes |
| `tree` | List directory contents in a tree-like format | Connector-glyph layout with `--charset` UTF-8/ASCII, `-a` hidden entries, `-d` dirs only, `-L` depth, repeatable `-P`/`-I` patterns (`*`, `?`, `[set]`, `[^set]`, ranges, `|` alternation), `-f` full-path prefixes, `-i` indent-free output, `-F` type indicators, `-s`/`-h`/`-p`/`-u`/`-g`/`-D` (UTC) columns, `--dirsfirst`/`-r`/`-t` sorting, `-C` colorized output, JSON (`-J`), XML (`-X`) and HTML (`-H`) output, `--noreport` count suppression, and an `-o` that stages before it publishes |
| `mkdir` | Create directories | Supports `-p` for nested creation, multiple directories in one call |
| `rm` | Remove files or directories | Supports `-r` recursive, `-f` force, combined `-rf` |
| `cp` | Copy files and directories | Supports `-r` recursive, `-f` force, multi-source to directory |
| `mv` | Move (rename) files and directories | Atomic rename with automatic cross-device fallback (copy + delete) |
| `touch` | Create empty files or update timestamps | Creates new files, updates mtime/atime on existing files |
| `head` | Output the first part of files | `-n` lines, `-c` bytes, multi-file with headers, stdin pipe |
| `tail` | Output the last part of files | `-n` lines, `-c` bytes, ring buffer for efficiency, stdin pipe |
| `grep` | Search for patterns in files or stdin | `-i` ignore case, `-v` invert, `-n` line numbers, `-c` count, `-j`/`--threads` parallel multi-threaded search |
| `chmod` | Change file mode bits | Octal numeric mode, `-R` recursive directory traversal |
| `chown` | Change file owner and group | POSIX `user[:group]` syntax, `-R` recursive, numeric ID or name |
| `chgrp` | Change group ownership | Group name or numeric GID, `-R` recursive |
| `df` | Report file system disk space usage | Parses `/proc/mounts` + `statvfs` syscall, `-h` human-readable, per-path query |
| `du` | Estimate file space usage | `-h` human-readable, `-s` summarize, `-d` max-depth control |
| `ps` | Report a snapshot of current processes | Parses `/proc/[pid]/stat` + `cmdline`, `-e`/`-A` all processes, `-o` custom columns |
| `kill` | Send signals to processes | POSIX signal FFI, supports signal names (`-TERM`) and numbers (`-9`), `-l` list signals |
| `free` | Display memory usage | Parses `/proc/meminfo`, `-h` human-readable, shows Mem + Swap |
| `uptime` | Tell how long the system has been running | Parses `/proc/uptime` + `/proc/loadavg`, shows uptime and 1/5/15 min load average |
| `ln` | Create links between files | `-s` symbolic links, `-f` force overwrite, hard links by default, multi-target to directory |
| `readlink` | Print resolved symbolic links | `-f`/`-e` canonicalize to absolute path, `-n` no trailing newline |
| `realpath` | Print canonical absolute paths | Existing-path canonicalization, quiet diagnostics, NUL-separated output |
| `sleep` | Pause for a specified duration | Fractional values, `s`/`m`/`h`/`d` suffixes, sums multiple intervals |
| `tee` | Copy stdin to files and stdout | Multiple output files, `-a` append, `-i` ignore interrupts, continues file output after a closed pipe |
| `tar` | Create, list, and extract tape archives | POSIX ustar headers, recursive directories, `-f` archives, `-z` Gzip streams, `-C` extraction directory, safe path validation |
| `gzip` | Compress or decompress Gzip streams | File and stdin operation, `-d`/`-k`/`-f`/`-c`, failure-safe output handling, pure-Rust DEFLATE |
| `gunzip` | Decompress Gzip files | `gzip -d`-compatible file naming and `-k`/`-f`/`-c` behavior |
| `zcat` | Decompress Gzip data to stdout | Reads `.gz` files or stdin without modifying source files |
| `unzip` | List and extract ZIP archives | Stored and Deflate entries, `-l`, `-o`, `-d`, CRC validation, Zip Slip protection |
| `md5sum` | Compute and check MD5 message digest | `-c` verification, `-b`/`-t` binary/text, `--status` quiet check |
| `sha1sum` | Compute and check SHA1 message digest | `-c` verification, `-b`/`-t` binary/text, `--status` quiet check |
| `sha256sum` | Compute and check SHA256 message digest | `-c` verification, `-b`/`-t` binary/text, `--status` quiet check |
| `sha512sum` | Compute and check SHA512 message digest | `-c` verification, `-b`/`-t` binary/text, `--status` quiet check |
| `b3sum` | Compute and check BLAKE3 message digest | Pure Rust parallelized hash chunking for large files, `-c` check |
| `uname` | Print system information | POSIX `uname()` FFI, `-a` all, `-s`/`-n`/`-r`/`-v`/`-m` individual fields |
| `test` / `[` | Evaluate conditional expressions | POSIX-compatible `test` and `[` forms, file/string/numeric tests, logical operators |
| `expr` | Evaluate expressions and print result | Arithmetic, comparison, logical, string ops; recursive descent parser |
| `find` | Search for files in a directory hierarchy | Glob `-name`, `-type`, `-maxdepth`, `-empty`, `-j`/`--threads` parallel directory walker; pure Rust traversal |
| `wc` | Print newline, word, and byte counts | 8 KiB streaming counter, `-l`/`-w`/`-c`/`-m`, `-j`/`--threads` parallel counting, multi-file `total`, stdin pipe |
| `sort` | Sort lines of text files | `-r` reverse, `-n` numeric, `-u` unique, multi-file merge |
| `uniq` | Report or omit repeated lines | Constant-memory group processing, optional output file, `-c`/`-d`/`-u`/`-i` |
| `cut` | Remove sections from each line | `-d` delimiter, `-f` fields, `-c` characters, range support |
| `tr` | Translate or delete characters | SET1/SET2 translation, `-d` delete, `-s` squeeze, range expansion |
| `id` | Print real and effective user and group IDs | `-u`/`-g`/`-G`/`-n` flags, query by user name, POSIX libc FFI |
| `whoami` | Print effective user name | POSIX `geteuid()` + `getpwuid()` FFI |
| `su` | Switch user | `-l` login shell, `-c` command, `-s` shell; root-only |
| `relax` | IdleBox special: take a break and relax | A unique relaxation experience, embodying the "Idle" spirit |
| `--install` | Automated applet launcher deployment | Previews with `--dry-run`, protects conflicts by default, and creates symlinks on Unix or `.exe` launchers on Windows |

---

## Quick Start

### Build

Requires Rust 1.85 or newer. The minimum toolchain is also validated on Alpine Linux/musl.

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
idlebox printf '%s = %04d\n' answer 42
idlebox env MODE=demo idlebox printenv MODE
idlebox cat -n README.md
idlebox ls --color=auto -lah
idlebox tar -czf source.tar.gz src
idlebox gzip -k report.txt
idlebox unzip archive.zip -d output

# Discover commands and version information
idlebox --help
idlebox help wc
idlebox --list
idlebox --version

# Automated install (create launchers for all applets)
idlebox --install              # Unix: /usr/local/bin; Windows: %LOCALAPPDATA%\IdleBox\bin
idlebox --install ./bin        # Install to a custom directory
idlebox --install --dry-run ./bin  # Preview without making changes
idlebox --install --force ./bin    # Explicitly replace conflicting files or links

# Via symlink on Unix
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

GitHub Actions separates formatting/lint/docs, native Linux/macOS/Windows tests, cross-target portability, and Linux release-size measurements into independent workflows.

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
