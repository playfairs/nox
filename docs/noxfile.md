# `noxfile` Reference

`noxfile` is optional and separate from `nox.build`. It is for named project tasks, not for declaring compilation targets.

## Syntax

The preferred syntax is a small YAML-like task map. A task can reference
another task with `@nox NAME`, run several commands, and use an indented
multiline command:

```yaml
tasks:
    format:
        run: cargo fmt --all
    rebuild:
        @nox clean
        @nox build
    package:
        @nox rebuild
        run: cargo build --release
        run: |
            mkdir -p {{build_dir}}
            echo version={{version}}
```

Run it with:

```sh
nox task format
```

## Current parser behavior

Nox recognizes `tasks:` at the top level, task names indented by two spaces, and task properties indented by at least four spaces. The `run:` value may be unquoted or enclosed in single or double quotes. Repeating `run:` executes commands in order. `run: |` collects all more-indented lines as one shell command. An `@nox NAME` line runs another task when it exists; otherwise it invokes the Nox command `NAME`.

The older block syntax remains accepted:

```text
task "format" {
    run = "cargo fmt --all"
}
```

On Unix, Nox executes the command as:

```text
sh -c "..."
```

On Windows, it executes the command through `windows-shell` when configured
in `nox.build`; otherwise it uses:

```text
cmd /C "..."
```

The task process exit status becomes the Nox command result. `{{version}}` is
the project version and `{{build_dir}}` is the selected build directory.

## Example

```yaml
tasks:
    format:
        run: cargo fmt --all
    check:
        run: cargo check
```

```sh
nox task format
nox task check
```

## `nox.build` settings

Task execution can use top-level settings from `nox.build`:

```text
set windows-shell := ["powershell.exe", "-NoProfile", "-Command"]
version := `cargo xtask`
```

Both `set NAME := VALUE` and `NAME := VALUE` are accepted. Backtick values
are evaluated when `nox.build` is parsed. String settings are available as
`{{NAME}}` in task commands.

Tasks are not part of the compilation build graph. Task arguments, task
outputs, and parallel task scheduling are not implemented.

For compilation, always use `nox.build`; do not put compiler commands in a task and expect Nox to provide incremental builds or dependency tracking.
