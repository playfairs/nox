TODO
====

This file tracks work that is still outstanding. Completed work is recorded in
``ROADMAP.rst`` with the commit that implemented it.

Additions
---------

* Improve the ``Run`` feature

	* Extend integration coverage for project runs, named target selection,
	  runtime and compile-and-run handlers, forwarded arguments, non-zero exit
	  codes, missing dependencies, and temporary-artifact cleanup.
	* Let projects declare an explicit default executable instead of relying on
	  the first executable target.
	* Make build directory, configuration, environment, working directory, and
	  target-specific run options configurable.
	* Improve diagnostics for missing setup state, unsupported file types,
	  compiler failures, runtime failures, and signal-terminated processes.
	* Review temporary compilation for platform-specific flags, linker
	  dependencies, source-relative includes, and parallel runs.

The following parts of the feature are implemented: project, target, and
standalone-source execution; F#, Python, JavaScript, Ruby, C, C++, and D run
handlers; argument and child-exit-code forwarding; temporary executable
cleanup; and run-handler loading from NOML rules.

* Extend variables and ``let`` bindings

	* Add target-local bindings and define whether shadowing is allowed.
	* Add explicit environment-variable access and command-line overrides only
	  after documenting precedence and reproducibility guarantees.
	* Add string interpolation, forward references, cycle detection, and
	  source-aware diagnostics if the language needs those capabilities.
	* Expand parser tests for metadata references, glob arguments, nested lists,
	  invalid types, undefined names, and binding scope.

Project-level immutable ``let NAME = VALUE`` declarations are implemented for
strings, booleans, and lists. References work in project metadata and target
properties, including sources, dependencies, include directories, defines,
compiler/linker flags, and install settings. Duplicate-binding diagnostics and
scalar, boolean, and list type checks are also implemented.


* Add a bounded expression language for computed configuration
	* Support numeric literals and arithmetic operators (`+`, `-`, `*`, `/`, `%`) with
	  parentheses and documented precedence, for example `let jobs = cpu_count / 2`.
	* Support only deterministic evaluation during configuration. This is intended for
	  practical build calculations, not arbitrary loops, recursion, shell execution,
	  or general-purpose programming-language behavior.
	* Define numeric types and conversion rules, including division by zero, fractional
	  results, overflow, negative values, and where integer values are required.
	* Allow expressions to consume let-bindings and a small set of explicitly defined
	  build inputs such as detected CPU count, host/target platform, and configuration.
	* Add useful built-ins only when they have stable cross-platform behavior; avoid
		making the build language accidentally dependent on the host shell.
	* Add evaluation tracing or clear diagnostics showing the expression and resolved
	  value when configuration fails.

* Add a package manager, initially for C and C++ dependencies
	* Extend the project model with declared external dependencies containing at least
	  a package name, version requirement, source/registry, and requested features.
	* Resolve packages transitively and generate a lockfile containing exact versions,
	  sources, checksums, and toolchain/platform selections for reproducible builds.
	* Provide a local cache and an offline mode, with clear behavior for cache misses,
	  unavailable packages, and version conflicts.
	* Convert resolved packages into compiler include paths, defines, library search
	  paths, linker flags, and transitive target dependencies without requiring users
	  to hand-write those values in every target.
	* Support system libraries through explicit adapters such as `pkg-config`, while
	  keeping downloaded packages separate from libraries supplied by the OS.
	* Define package build/install isolation, platform-specific variants, native build
	  scripts, and how failures are reported. Do not execute package-provided commands
	  without a documented trust and confirmation model.
	* Add commands for dependency inspection, lockfile generation/update, cache
	  management, and verification, plus integration tests using a local test registry.
