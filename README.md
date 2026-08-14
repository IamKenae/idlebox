# IdleBox

A modern, lightweight BusyBox alternative written in Rust.

## Features

- **Zero dependencies**: Pure Rust standard library implementation
- **Minimal binary size**: ~300KB with release optimizations
- **POSIX compatible**: Drop-in replacement for common Unix utilities
- **Modular design**: Easy to extend with new applets
- **Symlink support**: Call applets directly via symlinks

## Building

```bash
# Debug build
cargo build

# Release build (optimized for size)
cargo build --release

# Check binary size
ls -lh target/release/idlebox
```

## Usage

### Direct invocation

```bash
idlebox echo "Hello, World!"
idlebox relax 10
idlebox list
```

### Via symlink

```bash
cd target/release
ln -s idlebox echo
ln -s idlebox relax

./echo "Hello via symlink!"
./relax 5
```

## Available Applets

| Applet | Description |
|--------|-------------|
| `echo` | Print text to standard output |
| `relax` | IdleBox special: take a break and relax |
| `list` | List all available applets |

## Adding New Applets

1. Create a new file in `src/applets/`
2. Implement the `Applet` trait
3. Export it in `src/applets/mod.rs`
4. Register it in `src/core/dispatcher.rs`

See `ARCHITECTURE.md` for detailed design documentation.

## License

Apache-2.0
