# `noxfile` Reference

`noxfile` is optional and separate from `nox.build`. It is for named project tasks, not for declaring compilation targets.

## Syntax

The preferred syntax is a small YAML-like task map:

```yaml
tasks:
    format:
        run: cargo fmt --all
```

Run it with:

```sh
nox task format
```

## Current parser behavior

Nox recognizes `tasks:` at the top level, task names indented by two spaces, and task properties indented by at least four spaces. The `run:` value may be unquoted or enclosed in single or double quotes. Commands must be on one line.

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

On Windows, it executes the command through:

```text
cmd /C "..."
```

The task process exit status becomes the Nox command result.

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

## Current limitations

The syntax accepts `depends` in example designs, but the current task runner does not parse or execute task dependencies. Tasks are not part of the build graph. Environment declarations, task arguments, task outputs, and parallel task scheduling are not implemented yet.

For compilation, always use `nox.build`; do not put compiler commands in a task and expect Nox to provide incremental builds or dependency tracking.
