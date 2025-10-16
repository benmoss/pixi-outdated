# pixi-outdated

A CLI tool to check for outdated dependencies in pixi projects.

## Features

- Check for outdated conda packages by querying channel repodata
- Check for outdated PyPI packages via the PyPI JSON API
- Leverages `pixi list` to get package information (no manual parsing!)
- Supports filtering by explicit dependencies only
- Can check specific packages or all dependencies
- Multi-environment and multi-platform support

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
pixi-outdated --platform linux-64
```

### Additional options

```bash
pixi-outdated --help

Arguments:
  [PACKAGES]...       Specific package names to check (if not provided, checks all packages)

Options:
  -x, --explicit                 Only check packages explicitly listed in pixi.toml
  -e, --environment <ENV>        The environment to check (defaults to the default environment)
  -p, --platform <PLATFORM>      The platform to check (defaults to the current platform)
  -j, --json                     Output in JSON format
  -v, --verbose                  Verbose output
  -f, --manifest <MANIFEST>      Path to the pixi.toml file
  -h, --help                     Print help
  -V, --version                  Print version
```

## Development Status

This tool is under active development. Current status:

- [x] CLI argument parsing with support for:
  - [x] Multiple package names
  - [x] Explicit dependencies only (`--explicit`)
  - [x] Environment selection (`--environment`)
  - [x] Platform selection (`--platform`)
  - [x] Verbose and JSON output modes
- [x] Integration with `pixi list --json`
- [x] JSON parsing for package information
- [ ] Conda repodata querying with rattler
- [ ] PyPI API integration
- [ ] Version comparison logic
- [ ] Pretty output formatting

## Architecture

Instead of parsing `pixi.toml` and `pixi.lock` directly, we shell out to `pixi list --json` to get package information. This approach:

- Avoids reinventing the wheel
- Leverages pixi's existing logic for resolving packages
- Automatically supports all pixi features (environments, platforms, etc.)
- Simplifies our codebase

```
src/
├── main.rs         # CLI entry point
├── lib.rs          # Library exports
├── pixi.rs         # Shell out to `pixi list --json`
├── conda.rs        # Query conda channels for latest versions
└── pypi.rs         # Query PyPI JSON API for latest versions
```

## License

MIT
