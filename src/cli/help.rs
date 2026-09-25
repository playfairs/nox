use crate::core::output;

pub fn print(command: &str) {
    let text = match command {
        "" => {
                "The Nox Build & Automation System\n\nUsage: nox <COMMAND> [OPTIONS]\n\nNox reads nox.build. Typical workflow:\n  nox setup build\n  nox compile -C build\n\nCommands:
  setup, configure      Configure and detect toolchains
  compile, build, b     Compile and link configured targets
  rebuild               Reconfigure and compile from scratch
  clean                 Remove generated build artifacts
  doctor, doc           Diagnose project setup and build state
  install, i            Build and install marked targets
  uninstall             Remove installed targets
  validate              Check nox.build and dependencies
  status, stat          Show configuration and toolchains
  riders                List supported language backends
  targets, list         List declared targets
  graph                 Show dependency order
  run, r                Compile and run a target
  nomlfmt               Format NOML files
  test                  Run the noxfile test task
  task                  Run a named noxfile task
  version               Print the Nox version
  update                Update Nox from the latest repository version
  bump-version, bump    Bump or set the project version
  help                  Show command-specific help

Options:\n  -h, --help              Show command help\n  -v, --version           Print the Nox version\n  -C, --build-dir DIR     Select a build directory\n  -j N                    Use N parallel jobs\n  --release               Use release configuration\n  --debug                 Enable full Rust backtraces for run/build failures\n  --compile-flag FLAG     Add a compiler flag\n  --reconfigure           Recreate the setup directory\n  --prefix PATH           Select the install prefix\n\nRun 'nox <COMMAND> --help' for details."
        }
        "setup" | "configure" => {
            "Usage: nox setup [BUILD_DIR] [OPTIONS]\n\nParse and validate nox.build, detect toolchains, and write build state to nox.state. Setup does not compile sources.\n\nThe default directory is build. Use --reconfigure to remove it first.\n\nExamples:\n  nox setup\n  nox setup build --release\n  nox setup build --reconfigure\n\nAliases: nox configure."
        }
        "build" | "compile" => {
            "Usage: nox compile [-C BUILD_DIR] [OPTIONS]\n\nLoad configured state, compile changed sources, and link targets in dependency order. Run nox setup first.\n\nThis is the explicit form: it points at the build directory you want to use, so it is useful when there are multiple configured build directories or when you want to be explicit.\n\nThe shorter forms are convenience aliases for the default configured build directory:\n  nox build\n  nox b\n\nThese are equivalent to:\n  nox compile -C build\n\nUse -j for parallel compilation and repeat --compile-flag for compiler options.\n\nExamples:\n  nox compile -C build\n  nox compile -C build -j 8\n  nox compile -C build --compile-flag -Wall\n  nox build\n  nox b\n\nAliases: nox build, nox b."
        }
        "rebuild" => {
            "Usage: nox rebuild [OPTIONS]\n\nDelete the selected build directory, configure it again, and compile all targets from scratch. Equivalent to setup --reconfigure followed by compile.\n\nExamples:\n  nox rebuild\n  nox rebuild --release"
        }
        "clean" => {
            "Usage: nox clean [-C BUILD_DIR]\n\nRemove generated state and artifacts from the selected build directory without deleting the root nox.state configuration. Installed files are preserved.\n\nExample:\n  nox clean -C build"
        }
        "doctor" | "doc" => {
            "Usage: nox doctor [-C BUILD_DIR]\n\nInspect the project, selected build directory, loaded configuration, and state files to help diagnose setup issues.\n\nExamples:\n  nox doctor\n  nox doc -C build"
        }
        "install" => {
            "Usage: nox install [OPTIONS]\n\nConfigure if needed, compile, and install targets marked install = true. Executables go to bin and libraries go to lib under the prefix.\n\nAliases: nox i.\n\nExamples:\n  nox install --release\n  nox i --prefix ~/.local"
        }
        "uninstall" => {
            "Usage: nox uninstall [--prefix PATH]\n\nRemove installed targets without compiling. The project definition determines which artifacts belong to Nox.\n\nExample:\n  nox uninstall --prefix ~/.local"
        }
        "validate" => {
            "Usage: nox validate\n\nParse nox.build without compiling. Check duplicate names, unknown dependencies, dependency cycles, and targets without sources."
        }
        "status" | "stat" => {
            "Usage: nox status [-C BUILD_DIR]\n\nShow whether the selected build directory is configured. Display project metadata, configuration, detected tools, compile flags, and target count.\n\nAliases: nox stat."
        }
        "riders" => {
            "Usage: nox riders\n\nList the language and toolchain Riders compiled into Nox. Riders map source extensions to compiler or interpreter backends."
        }
        "targets" | "list" => {
            "Usage: nox targets\n\nPrint every target declared in nox.build, one per line.\n\nAliases: nox list."
        }
        "graph" => {
            "Usage: nox graph\n\nPrint targets in dependency order. Dependencies appear before targets that consume them."
        }
        "run" => {
            "Usage: nox run [PATH|TARGET] [-- ARGS...]\n\nRun the current project, a named executable target, or a source file. Existing source files run directly without requiring nox.build or setup; direct-runtime files use their runtime, while C and C++ files are compiled to a temporary executable first.\n\nWith no path, '.' or a target name, Nox runs the initialized project.\n\nUse -- to forward arguments to the child process.\n\nExamples:\n  nox run\n  nox run .\n  nox run nox -- --version\n  nox run main.py\n  nox run demos/fsharp/Test.fsx\n  nox run demos/c/Test.c -- hello world\n\nAliases: nox r."
        }
        "nomlfmt" => {
            "Usage: nox nomlfmt <FILE|DIRECTORY>\n\nFormat one NOML file or recursively format every .noml file under a directory.\n\nExamples:\n  nox nomlfmt rules.noml\n  nox nomlfmt ."
        }
        "test" => {
            "Usage: nox test\n\nRun the task named test from noxfile. Equivalent to nox task test."
        }
        "task" => {
            "Usage: nox task NAME\n\nFind NAME in noxfile and execute its run command. YAML-style and legacy task syntax are supported."
        }
        "tasks" => {
            "Usage: nox tasks\n\nList available task names from noxfile in alphabetical order."
        }
        "version" => {
            "Usage: nox version\n\nPrint the installed Nox version and release channel. --version, -V, and -v are also accepted."
        }
        "update" => {
            "Usage: nox update [--dev | --channel CHANNEL | --version VERSION]\n\nUpdate Nox from the latest stable release on master. Use --dev or --channel dev for development releases, --channel stable for stable releases, or --version for an exact release. Nix-managed installations must be updated through Nix."
        }
        "bump-version" | "bump" => {
            "Usage: nox bump-version [major|minor|patch|VERSION]\n\nBump the VERSION file and update matching version references in project files. Defaults to a patch bump.\n\nAliases: nox bump."
        }
        "help" => {
            "Usage: nox help [COMMAND]\n\nShow general help or detailed help for one command. Every command accepts --help and -h."
        }
        "init" => {
            "Usage: nox init [PROJECT_NAME] [OPTIONS]\n\nAnalyze and initialize a project without overwriting existing files.\n\nOptions: --name NAME, --language LANGUAGE, --type TYPE, --template NAME, --no-nix, --formatter, --no-noxfile"
        }
        _ => "Unknown command. Run 'nox --help' to list available commands.",
    };
    output::help(text);
}
