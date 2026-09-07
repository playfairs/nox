# Architecture

Nox separates project description, graph validation, toolchain selection, action execution, and task automation.

## Source modules

- `src/main.rs`: process entry point and top-level error reporting.
- `src/cli.rs`: argument parsing and command orchestration.
- `src/parser.rs`: lexer, `nox.build` parser, and simple glob expansion.
- `src/rider.rs`: language/toolchain Rider registry and source-extension classification.
- `src/model.rs`: `Project`, `Target`, and `TargetKind` data structures.
- `src/graph.rs`: duplicate-name checks, missing dependency checks, cycle detection, and dependency ordering.
- `src/state.rs`: persistent `build/nox.state` configuration state.
- `src/toolchain.rs`: executable detection for C, C++, and Rust tools.
- `src/executor.rs`: compilation, linking, archiving, incremental checks, and parallel source workers.
- `src/task.rs`: optional `noxfile` task lookup and process execution.

## Riders

A Rider is Nox's language/toolchain backend concept. Riders are the parts of Nox that recognize a language's source files and connect them to compiler or linker behavior while the core project model and dependency graph remain language-agnostic.

The Rider catalog currently contains these built-in Riders:

- C Rider for `.c` sources
- C++ Rider for `.cpp`, `.cc`, and `.cxx` sources
- Rust Rider for `.rs` sources

Go, Java, C#, Swift, Zig, Python, JavaScript, TypeScript, and Kotlin also have direct backend action paths. Their toolchains are detected when the target is built, so a missing compiler or interpreter produces a specific toolchain error.

Use `nox riders` to list the available Riders. Adding a new language should add a Rider entry and keep language-specific behavior at the toolchain/executor boundary rather than adding language assumptions to graph traversal.

## Setup phase

`nox setup build` performs these operations:

1. Reads `build/nox` from the current project root.
2. Lexes and parses the project into a `Project`.
3. Validates duplicate targets, missing dependencies, and dependency cycles.
4. Detects a C-compatible compiler, linker, and archiver.
5. Creates the build directory.
6. Writes `build/nox.state` with the project root, build directory, configuration, compiler, linker, and archiver.

Setup does not compile source files.

## Build phase

`nox build build` loads `nox.state`, reparses `nox.build`, validates the graph, and visits targets in dependency order.

For C and C++ targets, each source becomes a compile action with:

- source input
- object output
- compiler executable
- compiler flags
- include directories
- preprocessor definitions
- dependency-file output

After all source actions finish, the target gets a link or archive action. Static libraries use the detected archiver; shared libraries and executables use the compiler driver as linker.

Rust targets follow a separate backend branch and invoke `rustc` directly on the first source file.

## Dependency ordering

The graph uses depth-first traversal with three states represented by visited and visiting sets. A dependency is emitted before its consumer. Encountering a name already in the visiting set is a cycle error.

Target-level ordering is dependency-aware. Source files within one C/C++ target are dispatched to worker threads, up to the `-j` limit.

## Incremental compilation

Each C/C++ source produces an object file and a compiler-generated `.d` file. An object is rebuilt when:

- the object does not exist;
- the source is newer than the object;
- the dependency file does not exist; or
- a recorded dependency path is newer than the object.

Nox does not implement a C preprocessor. It delegates header dependency discovery to the compiler using `-MMD -MF`.

The current Rust backend does not have equivalent per-source incremental dependency tracking; it invokes `rustc` for the target.

## Build state and configurations

Artifacts are separated by configuration:

```text
build/
  debug/
  release/
```

The state file records one active configuration for a build directory. `install --release` reconfigures the build directory when it previously held debug state.

## Process execution

Compiler, linker, archiver, and `rustc` are launched with `std::process::Command` and argument vectors. The core build path does not construct shell command strings. This keeps normal compilation independent of Unix shell syntax.

`noxfile` tasks are different: task `run` values are intentionally executed through `sh -c` on Unix and `cmd /C` on Windows because they are task automation commands.

## Installation

Only targets with `install = true` are copied. The destination is selected from the prefix:

- executable, Rust executable: `<prefix>/bin/<target>`
- static library: `<prefix>/lib/<target>`
- shared library: `<prefix>/lib/<target>`

The default Unix prefix is `/usr/local`. Use `--prefix` for a user-owned location.
