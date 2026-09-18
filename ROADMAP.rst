Nox Roadmap
===========

This is an implementation history, rather than a list of intentions. It
records the capabilities currently represented by the repository and the
commit that introduced them. Dates are commit dates in the repository history.
Maintenance-only version bumps, formatting-only commits, and ambiguous scratch
commits are omitted unless they mark a user-visible milestone.

2026-09-07: Initial Nox foundation
----------------------------------

* ``abbe434`` - Established the initial Nox build system: project and target
  parsing, the build graph, executor, state handling, CLI, toolchain layer,
  task support, error handling, Nix packaging, and the first CLI tests.

2026-09-07: Build foundation
-----------------------------

* ``3424107`` - Extended ``nox.build`` project metadata and the project model.
* ``8a9ac11`` - Added Rider backends and build workflows.
* ``de4f324`` - Added the YAML-like ``noxfile`` task syntax.
* ``39b8b4b`` - Added D executable targets.
* ``2335b38`` - Added C++ executable targets.
* ``dbb9361`` - Allowed source files without an associated Rider.
* ``7429b29`` and ``e1fce27`` - Exposed Nox through the repository and
  development Nix flakes.

2026-09-08: Execution, configuration, and bindings
---------------------------------------------------

* ``db5cd55`` - Organized the CLI and build output.
* ``6bb138b`` - Added the extensible run subsystem, including language
  handlers, temporary compilation, argument forwarding, and child-process
  status propagation.
* ``7c90ac7`` - Added runnable language examples used to exercise run
  behavior; ``01e7215`` added malformed-run fixtures.
* ``6c33f73`` - Added the configuration system.
* ``65f87df`` - Added project-level immutable bindings and integration tests.
  Bindings support strings, booleans, and lists, with references in project
  metadata and target properties plus duplicate and type diagnostics.
* ``d5e9648`` and ``8774cfa`` - Added the version-bump command and reporting
  of changed files.
* ``cc2a3b7`` - Taught Nox to run its own repository tasks.

2026-09-09: CLI and task ergonomics
------------------------------------

* ``2068073`` - Added short aliases for build and run commands.
* ``2722c99`` - Improved help text and alias details.
* ``637ba12`` - Added branded CLI output and the hyperlink banner.
* ``5552263`` - Expanded task support in ``noxfile``, including task
  dependencies, repeated commands, multiline commands, interpolation, and
  recursive task execution.

2026-09-11 to 2026-09-15: Project initialization and discovery
---------------------------------------------------------------

* ``37afd70`` - Added the project initialization command.
* ``32346e2`` - Documented the initialization system.
* ``a2d24b9`` - Added Haskell project support.
* ``8fd3a48`` - Improved project discovery and initialization behavior.
* ``3f08a36`` - Made initialization-system file ordering deterministic.
* ``59a2604`` - Added data-driven initialization rules.
* ``4ebff92`` - Added license detection from project files.
* ``155489e`` - Allowed global commands outside projects.
* ``679e4c6`` - Added standalone source-file execution with ``nox run``.

2026-09-15: NOML integration
-----------------------------

* ``a85fb29`` - Integrated NOML rulesets.
* ``0a73643`` - Integrated the local NOML core for rules parsing.
* ``66c894c`` - Added GitHub NOML resolution.
* ``c31ed85`` - Organized demos by language.

These changes made run handlers and build rules data-driven and moved the
repository toward NOML-backed configuration instead of hard-coded behavior.

2026-09-16: Targets, tooling, and current syntax
------------------------------------------------

* ``490a77b`` - Modernized executable target declarations.
* ``26cf55e`` - Added project selection for commands.
* ``049b0da`` - Added standalone NOML tooling.
* ``2587d2b`` - Added support for ``UNLICENSE`` project files.
* ``5b23976`` - Formatted embedded NOML rules.
* ``3b9fcd4`` - Added the root ``nomlfmt`` command.
* ``f43098c`` - Converted run handlers to NOML.
* ``8366c10`` - Converted the YAML-like task representation to NOML.

NOTES
--------------

Planned features remain on ``TODO.rst``, when a feature is implemented
it will be added to this file with it's commit and date-of-imeplementation.