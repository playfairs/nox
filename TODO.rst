Todo List
===============================

Additions
---------

* Improve the partially implemented "Run" feature
	* Already implemented:
		* `nox run [PATH|TARGET] [-- ARGS...]` can run the current project, a named
		  executable target, or a source file.
		* Runtime handlers exist for F# scripts, Python, JavaScript, and Ruby.
		* C, C++, and D source files are compiled to a temporary executable, run,
		  and cleaned up automatically.
		* Program arguments and child exit codes are forwarded by the command.

	* Remaining improvements:
		* Implement the registered but currently unsupported handlers for Rust, Go,
		  Java, C#, Swift, Zig, TypeScript, and Kotlin, or remove handlers until
		  their execution behavior is supported.
		* Add integration tests for project runs, named target selection, runtime
		  handlers, compile-and-run handlers, forwarded arguments, non-zero exit
		  codes, missing dependencies, and temporary-artifact cleanup.
		* Improve project-target selection so projects with multiple executable
		  targets can declare an explicit default instead of relying on the first
		  executable target.
		* Make run behavior more configurable for build directory, configuration,
		  environment variables, working directory, and target-specific run options.
		* Improve diagnostics for missing setup state, unsupported file types,
		  compiler failures, runtime failures, and signal-terminated processes.
		* Review temporary compilation behavior for platform-specific compiler
		  flags, linker dependencies, source-relative includes, and parallel runs.

* Extend the partially implemented variables and let-bindings feature
	* Implemented: project-level immutable `let NAME = VALUE` declarations for
	  strings, booleans, and lists.
	* Implemented: references from project metadata and target properties including
	  sources, dependencies, include directories, defines, compiler/linker flags,
	  and install settings.
	* Implemented: duplicate-binding diagnostics and type checks when a binding is
	  used as a scalar, boolean, or list.

	* Add target-local bindings and define whether shadowing is allowed. Project
	  bindings should remain visible to targets, while target bindings must not leak
	  to sibling targets.
	* Add explicit environment-variable access and command-line overrides only if
	  their precedence and reproducibility guarantees are documented.
	* Add string interpolation, forward references, cycle detection, and
	  source-aware diagnostics if the language needs those capabilities.
	* Expand parser tests for metadata references, glob arguments, nested lists,
	  invalid types, undefined names, and binding scope.

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
