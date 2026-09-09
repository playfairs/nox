====
nox
====

The Nox Build System.

.. image:: https://raw.githubusercontent.com/playfairs/nox/master/assets/icon.png
   :width: 80px
   :align: center

.. raw:: html

   <h2 align="center">The Nox Build System</h2>

Nox is a cross-platform build system and task runner written in Rust. Its core
models projects, targets, dependencies, toolchains, Riders, and structured
build actions independently of any one language.

See the `complete documentation <docs/README.md>`_ for project integration,
command behavior, the ``nox.build`` language, architecture, and
troubleshooting.

Build Nox
=========

.. note::

   Build Nox. It is a build system. The name does most of the work here.

.. code-block:: sh

   cargo build --release

Use Nox
=======

A project declares its build graph in ``nox.build``:

.. code-block:: text

   project "example" {
       version = "1.0.0"

       executable "example" {
           sources = glob("src/*.c")
           include_dirs = ["include"]
           defines = ["FEATURE=1"]
           flags = ["-Wall", "-Wextra"]
       }
   }

Configure once, then build incrementally:

.. code-block:: sh

   nox setup build
   nox build build -j8

Install the configured project into ``/usr/local``, placing executables in
``/usr/local/bin``:

.. code-block:: sh

   nox install

Use ``--prefix`` for a different installation root. ``nox install`` configures
and builds automatically when the build directory does not exist:

.. code-block:: sh

   nox install --prefix "$HOME/.local"

To install Nox itself from a source checkout, bootstrap the executable once
with Cargo, then let Nox handle the rest:

.. code-block:: sh

   cargo run -- install
   /usr/local/bin/nox

The default configuration is ``debug``; pass ``--release`` during setup for
release artifacts. Commands include ``setup``, ``configure``, ``build``,
``clean``, ``rebuild``, ``install``, ``uninstall``, ``validate``,
``status``/``stat``, ``test``, ``run``, ``graph``, ``targets``, ``list``,
``version``, and ``task NAME``. Every command supports ``--help``.

``noxfile`` is optional and contains task automation. It is separate from
``nox.build``, which describes the actual compilation graph.

Supported Backends
==================

C and C++ sources use the detected GCC or Clang-compatible compiler with
compiler-generated dependency files. Static libraries, shared libraries,
executables, includes, definitions, compiler flags, linker flags, debug/release
settings, and parallel compilation are supported. Basic Rust executables and
libraries use ``rustc`` directly.

Nix
===

The flake provides the package, development shell, formatter, and flake checks:

.. code-block:: sh

   nix develop
   nix fmt
   nix flake check

Other flakes can consume the repository package:

.. code-block:: nix

   inputs.nox.url = "github:playfairs/nox";

   packages = [ inputs.nox.packages.${system}.default ];

The same package is available as
``inputs.nox.packages.${system}.nox``, and the executable can be run with:

.. code-block:: sh

   nix run github:playfairs/nox

See `nox-vscode <https://github.com/playfairs/nox-vscode>`_ for LSP support for
Visual Studio Code.