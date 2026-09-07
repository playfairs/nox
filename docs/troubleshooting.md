# Troubleshooting

## `build is not configured`

If `nox build` reports that the build directory is not configured, run setup first:

```sh
nox setup build
nox build build
```

A source edit does not require setup. A configuration change, build-directory change, or toolchain change should be followed by setup or rebuild.

## `nox install` cannot write `/usr/local`

Use elevated permissions when appropriate:

```sh
sudo nox install
```

Or choose a user-owned prefix:

```sh
nox install --prefix "$HOME/.local"
```

Ensure `$HOME/.local/bin` is on `PATH` if you want to invoke installed executables by name.

## Compiler not found

Setup detects the first available program from these lists:

- C/linker: `cc`, `clang`, `gcc`
- C++: `c++`, `clang++`, `g++`
- archive: `ar`, `llvm-ar`
- Rust: `rustc`

Install a compiler and ensure its executable is available on `PATH`.

## A header change did not rebuild

Inspect the target's `.d` file under `build/<configuration>/<target>/`. Nox uses compiler-generated dependency paths. Use project-relative include paths and verify the compiler emitted the expected dependency entry.

## A target name is unknown

Dependencies must refer to target names declared in the same `project` block. Check:

```sh
nox targets
nox graph
```

## A dependency cycle is reported

The target graph must be acyclic. For example, `a` depending on `b` while `b` depends on `a` cannot be ordered.

## `run` cannot find an executable

Build an executable target and pass its exact target name:

```sh
nox run app
```

The current runner is intended for executable targets and uses the target's build output path.

## Rust target behavior

Rust targets currently invoke `rustc` directly and use the first listed source. Cargo manifests, crate dependency resolution, and Rust incremental compilation are not used by Nox's Rust backend.

## Cleaning

`nox clean` removes the configured build directory, not the install prefix. Remove installed files separately if required.
