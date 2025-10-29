# pixi-outdated

A CLI tool to check for outdated dependencies in pixi projects.

## Installation

While our PR to get this package added to conda-forge is pending, you can install pixi-outdated with pixi via:

```bash
pixi global install --git https://github.com/benmoss/pixi-outdated
```

It can also be installed by downloading the artifact from the latest release and adding it to your PATH.

## Usage

Check all packages:

```bash
pixi-outdated
```

Check only explicit dependencies from pixi.toml:

```bash
pixi-outdated --explicit
```

Check specific packages:

```bash
pixi-outdated datasette
pixi-outdated datasette cowsay sqlite
```

Check packages in a specific environment:

```bash
pixi-outdated --environment prod
```

Check packages for a specific platform:

```bash
pixi-outdated --platform linux-64
```

By default, pixi-outdated checks all platforms in your pixi.lock file and groups results by platform.

### Options

```
Arguments:
  [PACKAGES]...       Specific package names to check

Options:
  -x, --explicit                 Only check packages explicitly listed in pixi.toml
  -e, --environment <ENV>        The environment to check (defaults to default environment)
  -p, --platform <PLATFORM>      The platform to check (defaults to all platforms in lockfile)
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
```

## Testing

```bash
cargo test
```

## License

MIT
