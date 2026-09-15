use super::language::{Language, ProjectType};

pub fn starter(language: Language, project_type: ProjectType) -> (&'static str, &'static str) {
    match (language, project_type) {
        (Language::Rust, ProjectType::Library) => ("src/lib.rs", "pub fn run() {}\n"),
        (Language::Rust, ProjectType::Executable) => ("src/main.rs", "fn main() {}\n"),
        (Language::Haskell, _) => (
            "app/Main.hs",
            "module Main where\n\nmain :: IO ()\nmain = putStrLn \"hello from Nox\"\n",
        ),
        (Language::C, _) => (
            "src/main.c",
            "#include <stdio.h>\n\nint main(void) {\n    puts(\"hello from Nox\");\n    return 0;\n}\n",
        ),
        (Language::Cpp, _) => (
            "src/main.cpp",
            "#include <iostream>\n\nint main() {\n    std::cout << \"hello from Nox\\n\";\n}\n",
        ),
        (Language::D, _) => (
            "src/main.d",
            "import std.stdio;\n\nvoid main() {\n    writeln(\"hello from Nox\");\n}\n",
        ),
        (Language::Swift, _) => ("Sources/main.swift", "print(\"hello from Nox\")\n"),
        (Language::JavaScript, _) => ("src/index.js", "console.log(\"hello from Nox\");\n"),
        (Language::TypeScript, _) => ("src/index.ts", "console.log(\"hello from Nox\");\n"),
        (Language::Python, _) => ("src/main.py", "print(\"hello from Nox\")\n"),
        (Language::FSharp, _) => ("src/main.fsx", "printfn \"hello from Nox\"\n"),
        (Language::Unknown, _) => ("src/main.c", "int main(void) { return 0; }\n"),
    }
}
pub fn flake(language: Language, formatter_enabled: bool) -> String {
    let package = match language {
        Language::Rust => "rustc cargo rustfmt",
        Language::Haskell => "ghc cabal-install fourmolu",
        Language::C | Language::Cpp => "clang clang-tools",
        Language::D => "ldc",
        Language::Swift => "swift",
        Language::JavaScript => "nodejs",
        Language::TypeScript => "nodejs nodePackages.typescript",
        Language::Python => "python3",
        Language::FSharp => "dotnet-sdk",
        Language::Unknown => "clang",
    };
    let formatter = if formatter_enabled {
        " nixfmt-rfc-style"
    } else {
        ""
    };
    format!(
        "{{\n  inputs.nixpkgs.url = \"github:NixOS/nixpkgs/nixpkgs-unstable\";\n  outputs = {{ nixpkgs, ... }}: let\n    systems = [ \"x86_64-linux\" \"aarch64-linux\" \"x86_64-darwin\" \"aarch64-darwin\" ];\n  in {{\n    devShells = builtins.listToAttrs (map (system: {{ name = system; value = nixpkgs.legacyPackages.${{system}}.mkShell {{ packages = with nixpkgs.legacyPackages.${{system}}; [ nox {package}{formatter} ]; }}; }}) systems);\n  }};\n}}\n"
    )
}
pub fn formatter(language: Language) -> Option<(&'static str, &'static str)> {
    match language {
        Language::Rust => Some(("rustfmt.toml", "edition = \"2024\"\n")),
        Language::Haskell => Some(("fourmolu.yaml", "indentation: 2\n")),
        Language::C | Language::Cpp => {
            Some((".clang-format", "BasedOnStyle: LLVM\nIndentWidth: 4\n"))
        }
        Language::JavaScript | Language::TypeScript => Some((
            ".prettierrc",
            "{\n  \"semi\": true,\n  \"singleQuote\": false\n}\n",
        )),
        Language::Swift => Some((
            ".swift-format",
            "{\n  \"indentation\": { \"spaces\": 4 }\n}\n",
        )),
        _ => None,
    }
}
pub fn nix_formatter(language: Language) -> String {
    let package = match language {
        Language::Rust => "rustfmt",
        Language::Haskell => "fourmolu",
        Language::C | Language::Cpp => "clang-tools",
        Language::JavaScript | Language::TypeScript => "nodePackages.prettier",
        Language::Swift => "swift-format",
        _ => "nixfmt-rfc-style",
    };
    format!(
        "{{ pkgs }}: pkgs.writeShellApplication {{\n  name = \"project-format\";\n  runtimeInputs = [ pkgs.{package} ];\n  text = \"true\";\n}}\n"
    )
}
pub fn noxfile(language: Language) -> String {
    let command = match language {
        Language::Rust => "cargo fmt --all",
        Language::C | Language::Cpp => "clang-format -i $(find src -type f)",
        Language::JavaScript | Language::TypeScript => "npx prettier --write .",
        _ => "true",
    };
    format!(
        "tasks:\n    format:\n        run: {command}\n    check:\n        run: nox validate\n    build:\n        @nox check\n        run: nox build\n    test:\n        @nox build\n        run: true\n"
    )
}
pub fn readme(name: &str, nix: bool) -> String {
    let nix_line = if nix {
        "\nWith Nix installed, enter the development environment with `nix develop`.\n"
    } else {
        ""
    };
    format!(
        "# {name}\n\nA project built and automated with Nox.\n\n## Build\n\n```sh\nnox setup\nnox build\n```\n{nix_line}"
    )
}
pub fn gitignore(language: Language) -> &'static str {
    match language {
        Language::Rust => "target/\nbuild/\n",
        Language::Haskell => "dist-newstyle/\nbuild/\n",
        Language::JavaScript | Language::TypeScript => "node_modules/\ndist/\n",
        _ => "build/\n",
    }
}
pub fn swift_manifest(name: &str, project_type: ProjectType) -> String {
    let product = if project_type == ProjectType::Library {
        format!("products: [.library(name: \"{name}\", targets: [\"{name}\"])],")
    } else {
        String::new()
    };
    format!(
        "// swift-tools-version: 6.3\nimport PackageDescription\nlet package = Package(name: \"{name}\", {product} targets: [.executableTarget(name: \"{name}\", path: \"Sources\")])\n"
    )
}
