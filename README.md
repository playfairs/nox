# The Nox Build System

Nox is a cross-platform build system and task runner written in Rust. Its core models projects, targets, dependencies, toolchains, and structured build actions independently of any one language.

## Build Nox

```sh
cargo build --release
```

## Use Nox

A project declares its build graph in `nox.build`:

```text
project "example" {
    version = "1.0.0"

    executable "example" {
        sources = glob("src/*.c")
        include_dirs = ["include"]
        defines = ["FEATURE=1"]
        flags = ["-Wall", "-Wextra"]
    }
}
```

Configure once, then build incrementally:

```sh
nox setup build
nox build build -j8
```

The default configuration is `debug`; pass `--release` during setup for release artifacts. Commands include `clean`, `rebuild`, `install`, `test`, `run`, `graph`, `targets`, and `task NAME`.

`noxfile` is optional and contains task automation. It is separate from `nox.build`, which describes the actual compilation graph.

## Supported backends

C and C++ sources use the detected GCC or Clang-compatible compiler with compiler-generated dependency files. Static libraries, shared libraries, executables, includes, definitions, compiler flags, linker flags, debug/release settings, and parallel compilation are supported. Basic Rust executables and libraries use `rustc` directly.

## Nix

The flake provides the package, development shell, formatter, and flake checks:

```sh
nix develop
nix fmt
nix flake check
```
