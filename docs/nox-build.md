# `nox.build` Reference

`nox.build` is a small declarative language. It is not TOML, JSON, YAML, or a shell script.

## File structure

The file contains one project declaration:

```text
project "nox" {
    description = "The Nox Build System."

    rust_executable "nox" {
        sources = ["src/main.rs"]
        flags = []
        install = true
    }
}

```

Whitespace is insignificant. Nox supports C-style comments: `//` starts a line comment and `/* ... */` starts a block comment. The legacy `#` line comment is also accepted. Strings use double quotes. Statements do not require semicolons.

```text
// Project metadata
project "example" {
    /* The version is read from a project-relative file. */
    version = file("./VERSION")
}
```

The parser accepts words as well as quoted strings for names and values, but quoted strings are recommended.

## Project properties

### `project NAME`

Required. The project name is displayed during setup.

### `version = VERSION`

Optional. Defaults to the version in the repository `VERSION` file. A version can explicitly load a project-relative file with the native `file` expression:

```text
version = file("./VERSION")
```

Nox trims surrounding whitespace and rejects missing or empty version files. The expression is resolved relative to the directory containing `nox.build`.

### `description = DESCRIPTION`

Optional project description shown by `nox status`:

```text
description = "A cross-platform application and its supporting libraries."
```

The value is a string and may contain spaces. It does not affect compilation, linking, or installation.

### `license = LICENSE`

Optional SPDX-style project license identifier:

```text
license = "Unlicense"
```

The value is project metadata and is displayed by `nox status`.

### `edition = EDITION`

Optional Nox project-file edition identifier. This is Nox's own configuration-language edition, not the Rust compiler edition used to build Nox. It defaults to `1`:

```text
edition = "1"
```

The value is stored in the project model and is available to Riders when they select language-specific settings. Cargo's Rust edition remains an implementation detail in `Cargo.toml`.

### `let NAME = VALUE`

Project-level immutable bindings provide reusable configuration values. A
binding must be declared before it is referenced, and duplicate names are
rejected:

```text
project "example" {
    let source_files = ["src/main.c"]
    let include_paths = ["include", "third_party/include"]
    let warning_flags = ["-Wall", "-Wextra"]
    let should_install = true

    executable "app" {
        sources = source_files
        include_dirs = include_paths
        flags = warning_flags
        install = should_install
    }
}
```

Bindings currently support strings, booleans, and lists of values. References
can be used in project metadata and target properties wherever the referenced
type is accepted, including `sources`, `dependencies`, `include_dirs`,
`defines`, `flags`, `linker_flags`, and `install`. Paths in a binding remain
relative to the directory containing `nox.build`.

Bindings are currently project-scoped and immutable. Target-local bindings,
environment-variable access, command-line overrides, string interpolation, and
arithmetic expressions are not implemented yet.

### `dependencies = [DEPENDENCIES]`

Optional project-level dependency names:

```text
dependencies = ["zlib", "openssl"]
```

These are metadata today. Build-graph dependencies between declared Nox targets belong on a target with `dependencies = [...]` and are resolved by Nox.

## Target kinds

Every target must have at least one source.

### C executable

```text
executable "app" {
    sources = ["src/main.c"]
    install = true
}
```

### C static library

```text
static_library "math" {
    sources = ["src/math.c"]
    install = true
}
```

`static` is accepted as an alias for `static_library`.

### C shared library

```text
shared_library "support" {
    sources = ["src/support.c"]
    install = true
}
```

`shared` is accepted as an alias for `shared_library`.

### C++ executable or library

Use the same C target kinds. Sources ending in `.cpp`, `.cc`, or `.cxx` select the C++ compiler and linker.

```text
executable "app" {
    sources = ["src/main.cpp", "src/widget.cc"]
}
```

Use `cxx_executable` when the target should always use the C++ compiler and linker, regardless of source extension:

```text
cxx_executable "app" {
    sources = ["src/main.cpp"]
}
```

### Other language executables

The generic `executable` target selects its Rider from the source extension:

```text
executable "tool" {
    sources = ["main.go"]
}
```

The same form works for D, Java, C#, Swift, Zig, Python, JavaScript, TypeScript, and Kotlin sources. Each Rider invokes its own detected toolchain and chooses its own artifact format. D, Swift, and Zig produce native executables; Python and JavaScript targets are checked and packaged; TypeScript is transpiled to JavaScript; Java and Kotlin produce JAR files. The D Rider looks for `ldc2`, `dmd`, or `gdc`.

### Rust executable

```text
rust_executable "tool" {
    sources = ["src/main.rs"]
    flags = []
    install = true
}
```

### Rust library

```text
rust_library "common" {
    sources = ["src/lib.rs"]
}
```

Rust targets currently use the first source file and invoke `rustc` directly. They are not Cargo packages and do not yet model Rust crate dependencies.

## Target properties

### `sources`

A list of paths relative to `nox.build`:

```text
sources = ["src/main.c", "src/print.c"]
```

Or a single simple glob:

```text
sources = glob("src/*.c")
```

The current glob implementation supports one `*` and reads one directory. It does not recursively expand `**`, brace patterns, or multiple glob expressions.

### `dependencies` / `depends`

A list of target names:

```text
executable "app" {
    sources = ["src/main.c"]
    dependencies = ["support"]
}
```

Dependencies must name targets declared in the same project. Nox validates missing targets and cycles before building. Libraries are passed to executable link commands in dependency order.

### `include_dirs` / `includes`

Directories passed as `-I` compiler arguments:

```text
include_dirs = ["include", "third_party/widget/include"]
```

Paths are relative to `nox.build`.

### `defines`

Preprocessor definitions passed as `-D` arguments:

```text
defines = ["VERSION=1", "FEATURE_ENABLED"]
```

### `flags`

Additional compiler flags:

```text
flags = ["-Wall", "-Wextra", "-std=c11"]
```

For Rust targets these are passed directly to `rustc`.

### `linker_flags`

Additional linker flags for C/C++ shared libraries and executables:

```text
linker_flags = ["-pthread"]
```

### `install`

Boolean that controls whether `nox install` copies the target:

```text
install = true
```

It defaults to `false`.

## Complete C example

```text
project "calculator" {
    version = "1.0.0"

    static_library "math" {
        sources = glob("src/math/*.c")
        include_dirs = ["include"]
        flags = ["-Wall", "-Wextra"]
    }

    executable "calculator" {
        sources = ["src/main.c"]
        include_dirs = ["include"]
        dependencies = ["math"]
        flags = ["-Wall", "-Wextra"]
        install = true
    }
}
```

The graph is:

```text
math -> calculator
```

Nox compiles `math` sources, archives `libmath.a`, compiles `calculator`, and links the executable with the library.
