# Command Reference

A bare `nox` invocation prints help. Every command accepts `--help` and `-h`, for example `nox graph --help`. `nox help COMMAND` is equivalent. Commands are run from a project root unless a build directory contains state pointing to another root.

## `nox --help` and `nox version`

Print the command list or the installed version:

```sh
nox --help
nox help build
nox version
nox --version
nox -v
```

## `nox setup [BUILD_DIR]`

Parse and validate `nox.build`, detect the C toolchain, and write build state.

```sh
nox setup
nox setup build
nox setup build --release
nox setup build --reconfigure
nox setup build --build-dir build
```

The positional directory and `--build-dir` are equivalent for setup. The default directory is `build`.

Setup does not compile sources.

`nox configure` is an alias for `nox setup`.

## `nox compile [-C BUILD_DIR]`

Load configured state, construct the target order, compile changed sources, and link targets.

```sh
nox compile
nox compile -C build
nox compile -C build -j8
nox compile -C build --release
nox compile -C build --compile-flag -C --compile-flag opt-level=1
```

`-C PATH` and `--build-dir PATH` select the configured build directory. `-j8` and `-j 8` are both accepted. Repeat `--compile-flag FLAG` to pass additional flags to compiler invocations. `nox build [BUILD_DIR]` remains an alias for this command.

## `nox rebuild [--release]`

Delete the selected build directory, configure it again, and build the project.

```sh
nox rebuild
nox rebuild --release
```

## `nox clean`

Remove the selected build directory. It does not remove installed files under the install prefix.

## `nox uninstall [--prefix PATH]`

Remove targets marked `install = true` from the selected prefix without rebuilding:

```sh
nox uninstall
nox uninstall --prefix "$HOME/.local"
```

## `nox validate`

Parse and validate `nox.build` without compiling:

```sh
nox validate
```

This checks duplicate target names, missing dependencies, and dependency cycles.

## `nox status` / `nox stat`

Show the configured project, build directory, active configuration, detected tools, and target count:

```sh
nox status
nox stat
```

## `nox riders`

List the language and toolchain Riders available to the current Nox binary:

```sh
nox riders
```

Riders include C, C++, Rust, Go, Java, C#, Swift, Zig, Python, JavaScript, TypeScript, and Kotlin. Each Rider has a direct backend action path; builds report a clear toolchain error when the required compiler or interpreter is not installed.

## `nox install [--prefix PATH] [--release]`

Configure if needed, build the project, and copy targets with `install = true` into the installation prefix.

The default prefix is `/usr/local` on Unix-like platforms and `C:\\Program Files\\Nox` on Windows.

```sh
nox install
nox install --prefix "$HOME/.local"
sudo nox install --release
```

Executables go to `bin`; libraries go to `lib`.

## `nox run [PATH|TARGET] [-- ARGS...]`

Run either the current project, a named project target, or one source file. Nox selects a file handler from the extension. Runtime-backed files run directly; C and C++ files are compiled into a temporary directory, executed, and cleaned up automatically.

```sh
nox run .
nox run hello
nox run examples/fsharp/Test.fsx
nox run examples/c/Test.c -- hello world
nox run examples/cpp/Test.cpp -- hello world
nox run examples/python/arrays.py
nox run examples/javascript/arrays.js
nox run examples/ruby/arrays.rb
nox run examples/d/arrays.d
nox run examples/python/arguments.py -- red green blue
nox run examples/python/exit_status.py -- 42
```

Every `arguments` example accepts the same forwarded arguments:

```sh
nox run examples/c/arguments.c -- red green blue
nox run examples/cpp/arguments.cpp -- red green blue
nox run examples/python/arguments.py -- red green blue
nox run examples/javascript/arguments.js -- red green blue
nox run examples/ruby/arguments.rb -- red green blue
nox run examples/fsharp/arguments.fsx -- red green blue
nox run examples/d/arguments.d -- red green blue
```

The `--` separator tells Nox to stop parsing its own options. Everything after it is passed to the example as normal program arguments. For example, `red` becomes argument 1, `green` becomes argument 2, and `blue` becomes argument 3.

## `nox targets`

Print every declared target name.

`nox list` is an alias for `nox targets`.

## `nox graph`

Print target names in dependency order. Dependencies appear before the targets that consume them.

## `nox task NAME`

Find `task "NAME"` in `noxfile` and execute its `run` command.

```sh
nox task format
```

## Options

### `--release` and `--debug`

Select release or debug configuration for setup, rebuild, and install. Debug compilation uses `-g -O0`; release compilation uses `-O2` for C/C++.

### `-jN` and `-j N`

Limit concurrent source compilation workers. Linking remains dependency-ordered.

### `-C PATH` and `--build-dir PATH`

Select the build directory for `compile`, `build`, and `status`. This is useful when a project maintains multiple build trees.

### `--reconfigure`

Remove the selected build directory before running setup. This is useful when changing configuration or toolchain settings:

```sh
nox setup build --reconfigure
```

### `--compile-flag FLAG`

Add a flag to compiler invocations. Repeat the option for flags that require separate arguments, such as `-C opt-level=1` for Rust:

```sh
nox setup build --compile-flag -C --compile-flag opt-level=1
```

Colors are enabled automatically for interactive terminals and disabled for pipes, CI, and environments with `NO_COLOR` set.

### `--prefix PATH`

Select the installation root. This option applies to `install`.
