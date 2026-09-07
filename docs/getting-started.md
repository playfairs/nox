# Getting Started

## 1. Install Nox

From a Nox source checkout, bootstrap the executable with Cargo:

```sh
cargo build --release
```

You can then use the built binary directly:

```sh
./target/release/nox help
```

To install Nox itself through Nox into `/usr/local/bin`:

```sh
cargo run -- install --release
```

On systems where `/usr/local` requires elevated permissions, build first and run the install step with permission to write there:

```sh
cargo build --release
sudo ./target/release/nox install --release
```

For a user-local installation:

```sh
./target/release/nox install --release --prefix "$HOME/.local"
```

This installs the executable at `$HOME/.local/bin/nox`.

## 2. Add Nox to a project

Change to the root directory of the project. The root must contain a file named `nox.build`.

A minimal C project looks like this:

```text
project "hello" {
    version = "1.0.0"
    description = "A small command-line application."

    executable "hello" {
        sources = ["src/main.c"]
        flags = ["-Wall", "-Wextra"]
        install = true
    }
}
```

The source path is relative to the directory containing `nox.build`. Create the source file at `src/main.c`:

```c
#include <stdio.h>

int main(void) {
    puts("hello from Nox");
    return 0;
}
```

## 3. Configure once

```sh
nox setup build
```

This reads `nox.build`, validates target names and dependencies, detects the C toolchain, and writes the build state to `build/nox.state`.

Use a different build directory explicitly:

```sh
nox setup out/debug
```

Use release defaults:

```sh
nox setup build --release
```

The configuration is stored in the build state. Re-run setup when changing configuration or changing toolchain settings.

## 4. Build

```sh
nox build build -j8
```

The output is placed below the build directory and configuration:

```text
build/
  debug/
    hello/
      main.o
      main.d
      hello
```

The `.d` file is compiler-generated dependency information for C/C++ headers. The next build reuses `main.o` when neither the source nor a recorded header dependency is newer.

After editing a source file, run the same build command again:

```sh
nox build build
```

No setup step is required for normal source edits.

## 5. Run and inspect

List targets:

```sh
nox targets
```

Print dependency order:

```sh
nox graph
```

Build and run a target:

```sh
nox run hello
```

`run` builds the configured project first and then runs `build/<configuration>/hello/hello`.

## 6. Install the project

Mark a target with `install = true`, then run:

```sh
nox install
```

The default layout is:

```text
/usr/local/bin/hello
/usr/local/lib/libexample.a
```

Use a different prefix when you do not have permission to write `/usr/local`:

```sh
nox install --prefix "$HOME/.local"
```

Executables are installed under `<prefix>/bin`; static and shared libraries are installed under `<prefix>/lib`.

## 7. Add tasks optionally

Create a file named `noxfile` only if the project needs task automation:

```text
task "format" {
    run = "cargo fmt --all"
}

task "check" {
    run = "cargo check"
}
```

Run a task with:

```sh
nox task format
nox task check
```

The build graph remains in `nox.build`; tasks do not replace it.
