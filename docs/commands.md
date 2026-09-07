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
nox setup build --build-dir build
```

The positional directory and `--build-dir` are equivalent for setup. The default directory is `build`.

Setup does not compile sources.

`nox configure` is an alias for `nox setup`.

## `nox build [BUILD_DIR]`

Load configured state, construct the target order, compile changed sources, and link targets.

```sh
nox build
nox build build
nox build build -j8
nox build build --release
```

`-j8` and `-j 8` are both accepted. `--release` selects release configuration only when the build state was configured for release; run setup again when switching configurations.

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

## `nox run TARGET`

Build the project and execute the selected target.

```sh
nox run hello
```

The current runner uses the target name as the output executable name and is intended for executable targets.

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

### `--build-dir PATH`

Select the build directory. This is useful when a project maintains multiple build trees.

### `--prefix PATH`

Select the installation root. This option applies to `install`.
