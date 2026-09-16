# Nox Demos

The language folders contain small source examples that can be run directly
with Nox:

- `c/`
- `cpp/`
- `d/`
- `fsharp/`
- `javascript/`
- `python/`
- `ruby/`

The `rust/` folder is a complete generated Nox project. From that directory:

```sh
nox setup
nox build
nox run rust
```

From the repository root, source demos can be run with commands such as:

```sh
nox run demos/python/arguments.py -- red green blue
nox run demos/cpp/Test.cpp -- hello world
```