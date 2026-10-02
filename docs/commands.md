# Command Reference

A bare `nox` invocation prints help. Every command accepts `--help` and `-h`, for example `nox graph --help`. `nox help COMMAND` is equivalent. Commands are run from a project root unless a build directory contains state pointing to another root.

## `nox --help` and `nox version`

Print the command list or the installed version and release channel:

```sh
nox --help
nox help build
nox version
nox --version
nox -v
```

## `nox init [PROJECT_NAME]`

Analyze the current directory, or create a named project directory, and generate a safe Nox configuration. Existing files are preserved. Empty projects receive language-aware source and ecosystem files; existing projects are inspected for manifests, source languages, build systems, Nix, formatters, Git, tests, and existing Nox files.

```sh
nox init
nox init my-project --language rust --type executable
nox init my-project --language typescript --no-nix --no-noxfile
nox init --language c --formatter
```

Use `--name`, `--language`, `--type`, and `--template` to provide answers in automation. `--no-nix` and `--no-noxfile` disable those generated components; `--formatter` requests a supported language formatter. Generated files are written only when absent, so an existing `nox.build`, `noxfile`, `flake.nix`, package manifest, README, or formatter configuration is preserved.

## `nox bump-version [major|minor|patch|VERSION]`

Bump the project version and update its references. With no argument, the patch
component is incremented:

```sh
nox bump-version
nox bump-version minor
nox bump-version 2.0.0
```

The command always requires and updates `VERSION`. Add `version_files = [...]`
to `nox.build` to restrict the other files it updates. Without that property,
Nox scans the project files automatically.

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

## `nox riders [install <language>]`

List the language and toolchain Riders available to the current Nox binary, or install a Rider's toolchain:

```sh
nox riders
nox riders install rust
nox riders install ruby
```

Riders include C, C++, Rust, Haskell, Go, Java, C#, Q#, Swift, Zig, Python, JavaScript, TypeScript, Kotlin, Ruby, F#, Lua, and PHP. Lua and PHP, like Ruby and F#, can be run directly from source files. If Nix is installed or Nox itself is Nix-managed, `nox riders install <language>` immediately prints Nix profile commands (`nix profile add nixpkgs#...`) for supported nixpkgs tool choices and the profile destination; unavailable nixpkgs packages are omitted. It does not prompt for a selection or install anything. Choose and run the suggested command(s) yourself. Nox will not invoke another package manager in this case. Otherwise, it uses an installed Homebrew, APT, DNF, or Pacman package manager. If more than one compiler/tool or package-manager plan is available, Nox asks which one to use. Before running a native package-manager command, Nox displays the package, full command, and install prefix and asks `Proceed with installation? [Y/n]`. Enter or `y` confirms; `n` cancels. System package managers may require administrator privileges and install dependencies in addition to the selected package. Homebrew auto-update is disabled for the install command.

Installing Rust's compiler (`rustc`), Cargo, and `rustup` are separate choices where the package manager supports them. Some package managers provide `rustc` and Cargo together as one package. Nox uses package names for the detected manager; availability depends on enabled repositories, which Nox does not add or change. Installation errors are reported rather than hidden.

## `nox install [--prefix PATH] [--release]`

Configure if needed, build the project, and copy targets with `install = true` into the installation prefix.

The default prefix is `/usr/local` on Unix-like platforms and `C:\\Program Files\\Nox` on Windows.

```sh
nox install
nox install .
nox install --prefix "$HOME/.local"
sudo nox install --release
```

Executables go to `bin`; libraries go to `lib`.
The command also accepts a remote source reference for valid Nox projects hosted on GitHub or published in the public registry:

```sh
nox install github:user/repo
nox install pkgs:<package>
```

A GitHub source must resolve to a repository that contains a valid `nox.build` and passes the same project validation, build graph checks, and Rider checks as any local Nox project. A `pkgs:` install resolves the package name through the public registry and then installs that GitHub source through the same pipeline.

Successful local, GitHub, and registry installs are recorded in Nox's installed-package database. The database stores project metadata, installation method, source URL or local path, and the paths of installed artifacts.

## `nox packages [list|search QUERY|NAME]`

List installed packages or show information about an installed package. Use `search` to query the public registry.

```sh
nox packages
nox packages <package>
nox packages <package>
nox packages search <package>
```

`nox packages` and `nox packages list` read only the local installed-package database. `nox packages NAME` displays details for an installed package and reports an error if it is not installed. Only `nox packages search QUERY` fetches the public registry at `https://pkgs.noxbuild.cc/packages.json`.

The database is stored at `~/Library/Application Support/nox/installed-packages.json` on macOS, `$XDG_DATA_HOME/nox/installed-packages.json` on Linux (or `~/.local/share/nox/installed-packages.json` when `XDG_DATA_HOME` is unset), and `%APPDATA%\nox\installed-packages.json` on Windows. Set `NOX_PACKAGE_DB` to use a different database file.

## `nox uninstall [PACKAGE]`

Remove a package by name using its recorded installed artifact paths. This removes the package's files and database entries without rebuilding it. With no package name, `nox uninstall` keeps its existing behavior and uninstalls the current project according to its `nox.build`.

```sh
nox uninstall <package>
nox uninstall
```
## `nox run [PATH|TARGET] [-- ARGS...]`

Run either the current project, a named project target, or one source file. An existing source file runs standalone and does not require `nox.build` or `nox setup`; Nox selects a file handler from the extension. Runtime-backed files run directly; C and C++ files are compiled into a temporary directory, executed, and cleaned up automatically. With no path, `.`, or a target name, Nox runs the initialized project.

```sh
nox run
nox run .
nox run hello
nox run main.py
nox run demos/fsharp/Test.fsx
nox run demos/c/Test.c -- hello world
nox run demos/cpp/Test.cpp -- hello world
nox run demos/python/arrays.py
nox run demos/javascript/arrays.js
nox run demos/ruby/arrays.rb
nox run demos/d/arrays.d
nox run demos/python/arguments.py -- red green blue
nox run demos/python/exit_status.py -- 42
```

Every `arguments` example accepts the same forwarded arguments:

```sh
nox run demos/c/arguments.c -- red green blue
nox run demos/cpp/arguments.cpp -- red green blue
nox run demos/python/arguments.py -- red green blue
nox run demos/javascript/arguments.js -- red green blue
nox run demos/ruby/arguments.rb -- red green blue
nox run demos/fsharp/arguments.fsx -- red green blue
nox run demos/d/arguments.d -- red green blue
```

The `--` separator tells Nox to stop parsing its own options. Everything after it is passed to the example as normal program arguments. For example, `red` becomes argument 1, `green` becomes argument 2, and `blue` becomes argument 3.

## `nox nomlfmt <FILE|DIRECTORY>`

Format NOML files using the formatter provided by the embedded NOML crate. An
explicit file formats only that file. A directory is searched recursively for
`.noml` files; generated `.git`, `target`, and `build` directories are skipped.

```sh
nox nomlfmt rules.noml
nox nomlfmt .
nox nomlfmt src/rules
```

Each formatted file is reported as it is written. The command does not require
`nox setup` or a project build state.

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
