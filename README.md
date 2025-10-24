# pixi-outdated

A CLI tool to check for outdated dependencies in pixi projects.

## Features

- ✅ **Multi-platform analysis** - Check all platforms in your lockfile simultaneously
- ✅ **Smart caching** - Queries each package only once across all platforms
- ✅ **Conda packages** - Query channel repodata with rattler for accurate version info
- ✅ **PyPI packages** - Check Python packages via the PyPI JSON API
- ✅ **No parsing needed** - Leverages `pixi list --json` for package information
- ✅ **Flexible filtering** - Check explicit dependencies only or specific packages
- ✅ **Multi-environment** - Support for different pixi environments
- ✅ **Progress tracking** - Visual progress bars for multi-platform queries
- ✅ **JSON output** - Machine-readable output for automation

## Installation

```bash
cargo install --path .
```

## Usage

### Check all packages (including transitive dependencies)

```bash
pixi-outdated
```

### Check only explicit dependencies from pixi.toml

```bash
pixi-outdated --explicit
```

### Check specific packages

```bash
# Single package
pixi-outdated datasette

# Multiple packages
pixi-outdated datasette cowsay sqlite
```

### Check packages in a specific environment

```bash
pixi-outdated --environment prod
```

### Check packages for a specific platform

```bash
# Single platform
pixi-outdated --platform linux-64

# Check all platforms in lockfile (default behavior)
pixi-outdated
```

**Note:** When no platform is specified, `pixi-outdated` automatically checks all platforms defined in your `pixi.lock` file and shows:
- Updates common to all platforms
- Platform-specific updates

### Additional options

```bash
pixi-outdated --help

Arguments:
  [PACKAGES]...       Specific package names to check (if not provided, checks all packages)

Options:
  -x, --explicit                 Only check packages explicitly listed in pixi.toml
  -e, --environment <ENV>        The environment to check (defaults to the default environment)
  -p, --platform <PLATFORM>      The platform to check (if not specified, checks all platforms in lockfile)
  -j, --json                     Output in JSON format
  -v, --verbose                  Verbose output with debug logging
  -f, --manifest <MANIFEST>      Path to the pixi.toml file
  -h, --help                     Print help
  -V, --version                  Print version
```

### Example Output

```bash
$ pixi-outdated
Checking platforms: linux-64, osx-arm64

=== All Platforms ===
cowsay: 5.0 -> 6.1
libffi: 3.4.6 -> 3.5.2

=== Platform: osx-arm64 ===
icu: 73.2 -> 75.1
python: 3.12.12 -> 3.14.0

=== Platform: linux-64 ===
libsqlite: 3.50.1 -> 3.50.4
python: 3.12.11 -> 3.14.0

Analysis complete!
```

## Development Status

✅ **Production Ready!** This tool is fully functional and well-tested.

**Completed Features:**
- [x] **Multi-platform analysis** - Automatically checks all platforms in lockfile
- [x] **Smart query optimization** - Each package queried only once across platforms (50% reduction in API calls)
- [x] **CLI argument parsing** with support for:
  - [x] Multiple package names
  - [x] Explicit dependencies only (`--explicit`)
  - [x] Environment selection (`--environment`)
  - [x] Platform selection (`--platform`)
  - [x] JSON output (`--json`)
  - [x] Verbose output with structured logging (`--verbose`)
- [x] **Integration with pixi** - Uses `pixi list --json` for package info
- [x] **Conda support** - Repodata querying with rattler
  - [x] Shared Gateway instance for performance (7s first query, <7ms cached)
  - [x] Multi-platform queries in single API call
  - [x] NoArch and platform-specific packages
- [x] **PyPI support** - PyPI JSON API integration with automatic caching
- [x] **Progress tracking** - Visual progress bars for long operations
- [x] **Comprehensive tests** - 25 tests covering all functionality
  - [x] Unit tests for all modules
  - [x] Integration tests with real lockfiles
  - [x] Error handling and edge cases
- [x] **Result coalescing** - Intelligently groups updates by platform

**Performance:**
- ~50% reduction in API calls through intelligent caching
- PyPI packages queried once regardless of platform count
- Parallel platform analysis with single progress bar

## Architecture

### Design Principles

Instead of parsing `pixi.toml` and `pixi.lock` directly, we leverage existing tools:

- **Use `pixi list --json`** - Avoids reinventing package resolution logic
- **Use `rattler_lock`** - Read lockfiles for platform information
- **Smart caching** - Query each unique package once across all platforms
- **Parallel analysis** - Process all platforms simultaneously

### Module Structure

```
src/
├── main.rs         # CLI entry point and multi-platform orchestration
├── lib.rs          # Library exports (lockfile, conda, pixi, pypi)
├── lockfile.rs     # Read platforms from pixi.lock using rattler_lock
├── pixi.rs         # Shell out to `pixi list --json` for package info
├── conda.rs        # Query conda channels for latest versions (with multi-platform support)
└── pypi.rs         # Query PyPI JSON API for latest versions

tests/
└── integration_test.rs  # End-to-end tests with real lockfiles
```

### Key Optimizations

1. **PackageKey Deduplication**: Unique packages identified by `(name, channel, kind)` tuple
2. **Version Cache**: All version queries cached before building per-platform results
3. **Multi-platform Queries**: Conda packages queried across all platforms in one API call
4. **PyPI Caching**: Automatically cached since they have no channel (platform-independent)

## Testing

Run the test suite:

```bash
# All tests
cargo test

# With output
cargo test -- --nocapture

# Specific test
cargo test test_get_platforms_from_lockfile

# Integration tests only
cargo test --test integration_test
```

Run clippy:

```bash
pixi run cargo-clippy
```

### Test Coverage

- **17 unit tests** - Testing individual modules (conda, pixi, lockfile)
- **6 integration tests** - End-to-end testing with real lockfiles
- **2 binary tests** - Testing main application logic

## License

MIT
