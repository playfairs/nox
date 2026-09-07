# Developing Nox

## Build and test

```sh
cargo fmt --all
cargo test --all-targets
cargo build --release
```

The repository also provides a Nix development shell:

```sh
nix develop
nix fmt
nix flake check
```

## Self-hosting

Nox's own `nox.build` declares a Rust executable target named `nox`. After bootstrapping with Cargo, the project can configure, build, and install itself through Nox:

```sh
cargo run -- setup build
cargo run -- build build
cargo run -- install --prefix "$HOME/.local"
```

## Adding a target kind

To add a target kind, update the following boundaries together:

1. Add a `TargetKind` variant in `src/model.rs`.
2. Add parser recognition in `src/parser.rs`.
3. Add action generation and artifact naming in `src/executor.rs`.
4. Decide how setup discovers or records the toolchain in `src/toolchain.rs` and `src/state.rs`.
5. Add a graph/build integration test and document the syntax in `docs/nox-build.md`.

Keep language-specific command generation in the executor/toolchain boundary. The graph and project model should remain language-agnostic.

## Adding a CLI command

Add argument handling and dispatch in `src/cli.rs`, then update:

- the help output;
- `docs/commands.md`;
- README usage examples when the workflow is common;
- integration tests in `tests/`.

Commands that mutate or execute files should return an error for a failed process rather than printing a success message.

## Testing a project externally

Create a temporary project containing `nox.build`, run setup and build from that directory, and verify the produced artifact. Test at least:

- missing target dependencies;
- dependency cycles;
- source changes;
- header changes for C/C++;
- debug and release output separation;
- parallel compilation;
- custom installation prefixes.
