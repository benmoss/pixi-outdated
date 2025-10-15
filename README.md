# pixi-outdated

A CLI tool to check for outdated dependencies in pixi projects.

## Features

- Check for outdated conda packages by querying channel repodata
- Check for outdated PyPI packages via the PyPI JSON API
- Leverages rattler and pixi's own cache for efficiency
- Supports filtering by explicit dependencies only
- Can check specific packages or all dependencies

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

### Check a specific package
```bash
pixi-outdated datasette
```

### Additional options
```bash
pixi-outdated --help

Options:
  -e, --explicit       Only check packages explicitly listed in pixi.toml
  -j, --json          Output in JSON format
  -v, --verbose       Verbose output
  -f, --manifest      Path to the pixi.toml file (defaults to pixi.toml)
  -h, --help          Print help
  -V, --version       Print version
```

## Development Status

This tool is under active development. Current status:

- [x] CLI argument parsing
- [x] Basic project structure
- [ ] TOML/lock file parsing
- [ ] Conda repodata querying with rattler
- [ ] PyPI API integration
- [ ] Version comparison logic
- [ ] Pretty output formatting

## Architecture

```
src/
├── main.rs         # CLI entry point
├── lib.rs          # Library exports
├── parser.rs       # Parse pixi.toml and pixi.lock
├── conda.rs        # Query conda channels via rattler
└── pypi.rs         # Query PyPI JSON API
```

## License

MIT
