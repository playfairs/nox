# Nox Documentation

Nox is a Rust build system and task runner. This documentation describes the behavior implemented by the current binary, including the project file syntax, command workflow, build graph, toolchain selection, incremental compilation, installation, and extension points.

## Guides

- [Getting Started](getting-started.md): add Nox to a C, C++, or Rust project and build the first target.
- [Command Reference](commands.md): every CLI command and option currently supported.
- [`nox.build` Reference](nox-build.md): the declarative project file grammar and target properties.
- [`noxfile` Reference](noxfile.md): optional task automation syntax and current limitations.
- [Architecture](architecture.md): how parsing, configuration, graph ordering, actions, execution, and installation fit together.

## Important distinction

`nox.build` is the build definition. It declares the project, targets, source files, compiler settings, and dependencies.

`noxfile` is optional task automation. It contains named commands such as formatting or tests. A project does not need a `noxfile` to compile.

## Current scope

The current implementation supports:

- C executables, static libraries, and shared libraries
- C++ executables, static libraries, and shared libraries for `.cpp`, `.cc`, and `.cxx` sources
- Rust executables and libraries through `rustc`
- Built-in language/toolchain Riders for C, C++, Rust, Go, Java, C#, Swift, Zig, Python, JavaScript, TypeScript, and Kotlin
- GCC- and Clang-compatible C/C++ toolchains
- Debug and release configurations
- Dependency-aware target ordering
- Parallel compilation with `-jN`
- Compiler-generated C/C++ header dependency files
- Incremental object compilation
- Installation into `/usr/local` by default, or a custom `--prefix`

Some language and project-file features named in the long-term design are not yet implemented. The reference pages call those limitations out explicitly so that a project configuration can be written against the actual binary.
