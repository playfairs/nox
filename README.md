# nox

The Nox Build System.

---

Nox is a cross-platform build system and task runner written in Rust. Its core models projects, targets, dependencies, toolchains, Riders, and structured build actions independently of any one language.

See the [complete documentation](docs/README.md) for project integration, command behavior, the `nox.build` language, architecture, and troubleshooting.

## Build Nox
(haha get it, build nox? because its a build system?? no? ok.)

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

Install the configured project into `/usr/local`, placing executables in `/usr/local/bin`:

```sh
nox install
```

Use `--prefix` for a different installation root. `nox install` configures and builds automatically when the build directory does not exist:

```sh
nox install --prefix "$HOME/.local"
```

To install Nox itself from a source checkout, bootstrap the executable once with Cargo, then let Nox handle the rest:

```sh
cargo run -- install
/usr/local/bin/nox
```

The default configuration is `debug`; pass `--release` during setup for release artifacts. Commands include `setup`, `configure`, `build`, `clean`, `rebuild`, `install`, `uninstall`, `validate`, `status`/`stat`, `test`, `run`, `graph`, `targets`, `list`, `version`, and `task NAME`. Every command supports `--help`.

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

Other flakes can consume the repository package:

```nix
inputs.nox.url = "github:playfairs/nox";

packages = [ inputs.nox.packages.${system}.default ];
```

The same package is available as `inputs.nox.packages.${system}.nox`, and the executable can be run with `nix run github:playfairs/nox`.
