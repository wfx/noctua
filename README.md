# Noctua

An image viewer application for the COSMIC™ desktop

![Screenshot](docs/images/screenshot.png)


## Installation

A [justfile](./justfile) is included by default for the [casey/just][just] command runner.

- `just` builds the application with the default `just build-release` recipe
- `just run` builds and runs the application
- `just install` installs the project into the system
- `just vendor` creates a vendored tarball
- `just build-vendored` compiles with vendored dependencies from that tarball
- `just check` runs clippy on the project to check for linter warnings
- `just check-json` can be used by IDEs that support LSP

## Documentation

- [Keyboard Shortcuts](docs/usage.md)
- [Features](docs/features.md)
- [Development Guide](docs/development.md)

## License

GPL-3.0-or-later
