use crate::core::error::{Error, Result};
use crate::core::output;
use crate::project::analyzer::{self, Analysis, Language, ProjectType};
use std::fs;
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Options {
    pub project_name: Option<String>,
    pub name: Option<String>,
    pub language: Option<String>,
    pub project_type: Option<String>,
    pub template: Option<String>,
    pub nix: bool,
    pub formatter: bool,
    pub noxfile: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            project_name: None,
            name: None,
            language: None,
            project_type: None,
            template: None,
            nix: true,
            formatter: false,
            noxfile: true,
        }
    }
}

pub fn resolve_root(name: Option<&str>) -> Result<PathBuf> {
    let current = std::env::current_dir()?;
    let Some(name) = name else { return Ok(current) };
    let path = PathBuf::from(name);
    Ok(if path.is_absolute() {
        path
    } else {
        current.join(path)
    })
}

pub fn run(root: &Path, options: Options) -> Result<()> {
    if !root.exists() {
        fs::create_dir_all(root)?;
    }
    if !root.is_dir() {
        return Err(Error::Config(format!(
            "init path '{}' is not a directory",
            root.display()
        )));
    }
    output::action("analyzing", root.display());
    let analysis = analyzer::analyze(root)?;
    report(&analysis);
    let language = choose_language(&analysis, options.language.as_deref())?;
    let project_type = choose_type(&analysis, options.project_type.as_deref())?;
    let requested_name = options
        .name
        .clone()
        .or(options.project_name.clone())
        .map(|value| {
            Path::new(&value)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(&value)
                .to_string()
        });
    let name = requested_name
        .or(analysis.project_name.clone())
        .or_else(|| {
            root.file_name()
                .and_then(|value| value.to_str())
                .map(str::to_string)
        })
        .ok_or_else(|| Error::Config("could not infer a project name; use --name".to_string()))?;
    let options = Options {
        project_name: Some(name.clone()),
        ..options
    };
    generate(root, &analysis, language, project_type, &options)?;
    output::success(format!("Nox project '{name}' initialized successfully"));
    Ok(())
}

fn report(analysis: &Analysis) {
    for language in &analysis.languages {
        output::action("detected", language.label());
    }
    if !analysis.package_manifests.is_empty() {
        output::action(
            "detected",
            format!("{} package manifest(s)", analysis.package_manifests.len()),
        );
    }
    output::action(
        "found",
        format!("{} source file(s)", analysis.source_files.len()),
    );
    if analysis.has_tests {
        output::action("detected", "test suite");
    }
    if analysis.has_git {
        output::action("detected", "Git repository");
    }
    if analysis.has_flake {
        output::action("detected", "Nix flake");
    }
    if analysis.has_nox_build {
        output::action("detected", "nox.build");
    }
}

fn choose_language(analysis: &Analysis, requested: Option<&str>) -> Result<Language> {
    if let Some(value) = requested {
        return Language::parse(value)
            .ok_or_else(|| Error::Config(format!("unsupported language '{value}'")));
    }
    if analysis.languages.len() == 1 {
        return Ok(*analysis.languages.first().expect("one language exists"));
    }
    if analysis.languages.len() > 1 {
        return prompt_language(analysis);
    }
    if io::stdin().is_terminal() {
        prompt_language(&Analysis::default())
    } else {
        Ok(Language::Rust)
    }
}

fn prompt_language(analysis: &Analysis) -> Result<Language> {
    let default = analysis
        .languages
        .first()
        .copied()
        .unwrap_or(Language::Rust);
    print!("Language [{}]: ", default.label());
    io::stdout().flush()?;
    let mut value = String::new();
    io::stdin().read_line(&mut value)?;
    let value = value.trim();
    if value.is_empty() {
        return Ok(default);
    }
    Language::parse(value).ok_or_else(|| Error::Config(format!("unsupported language '{value}'")))
}

fn choose_type(analysis: &Analysis, requested: Option<&str>) -> Result<ProjectType> {
    if let Some(value) = requested {
        return ProjectType::parse(value)
            .ok_or_else(|| Error::Config(format!("unsupported project type '{value}'")));
    }
    if let Some(project_type) = analysis.project_type {
        return Ok(project_type);
    }
    if io::stdin().is_terminal() {
        print!("Project type [executable]: ");
        io::stdout().flush()?;
        let mut value = String::new();
        io::stdin().read_line(&mut value)?;
        return Ok(ProjectType::parse(value.trim()).unwrap_or(ProjectType::Executable));
    }
    Ok(ProjectType::Executable)
}

fn generate(
    root: &Path,
    analysis: &Analysis,
    language: Language,
    project_type: ProjectType,
    options: &Options,
) -> Result<()> {
    let name = options.project_name.as_deref().unwrap_or("project");
    let source = source_path(root, analysis, language, project_type);
    if analysis.source_files.is_empty() {
        write_if_absent(&source.0, &source.1)?;
    }
    if matches!(language, Language::Rust) && !root.join("Cargo.toml").exists() {
        let kind = if project_type == ProjectType::Library {
            "[lib]\npath = \"src/lib.rs\"\n"
        } else {
            "[[bin]]\nname = \"{name}\"\npath = \"src/main.rs\"\n"
        };
        write_if_absent(
            &root.join("Cargo.toml"),
            &format!(
                "[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n\n{kind}"
            ),
        )?;
    }
    if matches!(language, Language::JavaScript | Language::TypeScript)
        && !root.join("package.json").exists()
    {
        write_if_absent(
            &root.join("package.json"),
            &format!(
                "{{\n  \"name\": \"{name}\",\n  \"version\": \"0.1.0\",\n  \"private\": true\n}}\n"
            ),
        )?;
    }
    if language == Language::TypeScript && !root.join("tsconfig.json").exists() {
        write_if_absent(
            &root.join("tsconfig.json"),
            "{\n  \"compilerOptions\": {\n    \"target\": \"ES2022\",\n    \"module\": \"commonjs\",\n    \"outDir\": \"dist\",\n    \"strict\": true\n  },\n  \"include\": [\"src\"]\n}\n",
        )?;
    }
    if language == Language::Swift && !root.join("Package.swift").exists() {
        write_if_absent(
            &root.join("Package.swift"),
            &swift_manifest(name, project_type),
        )?;
    }
    if language == Language::Python && !root.join("pyproject.toml").exists() {
        write_if_absent(
            &root.join("pyproject.toml"),
            &format!(
                "[project]\nname = \"{name}\"\nversion = \"0.1.0\"\nrequires-python = \">=3.10\"\n"
            ),
        )?;
    }
    write_if_absent(
        &root.join("nox.build"),
        &nox_build(name, language, project_type, &source.0, root),
    )?;
    if options.noxfile && !analysis.has_noxfile {
        write_if_absent(&root.join("noxfile"), &noxfile(language))?;
    }
    if options.nix && !analysis.has_flake {
        write_if_absent(&root.join("flake.nix"), &flake(language, options.formatter))?;
    }
    if options.formatter && analysis.formatter_configs.is_empty() {
        if let Some((path, contents)) = formatter(language) {
            write_if_absent(&root.join(path), contents)?;
        }
    }
    if options.nix && options.formatter && !analysis.has_nix_formatter {
        fs::create_dir_all(root.join("nix"))?;
        write_if_absent(&root.join("nix/formatter.nix"), &nix_formatter(language))?;
    }
    if !analysis.has_readme {
        write_if_absent(&root.join("README.md"), &readme(name, options.nix))?;
    }
    if !root.join(".gitignore").exists() {
        write_if_absent(&root.join(".gitignore"), gitignore(language))?;
    }
    Ok(())
}

fn source_path(
    root: &Path,
    analysis: &Analysis,
    language: Language,
    project_type: ProjectType,
) -> (PathBuf, String) {
    if let Some(path) = analysis.source_files.first() {
        return (root.join(path), String::new());
    }
    let (relative, contents) = match (language, project_type) {
        (Language::Rust, ProjectType::Library) => ("src/lib.rs", "pub fn run() {}\n"),
        (Language::Rust, ProjectType::Executable) => ("src/main.rs", "fn main() {}\n"),
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
    };
    (root.join(relative), contents.to_string())
}

fn nox_build(
    name: &str,
    language: Language,
    project_type: ProjectType,
    source: &Path,
    root: &Path,
) -> String {
    let relative = source
        .strip_prefix(root)
        .unwrap_or(source)
        .to_string_lossy()
        .replace('\\', "/");
    let target = match (language, project_type) {
        (Language::Rust, ProjectType::Library) => "rust_library",
        (Language::Rust, ProjectType::Executable) => "rust_executable",
        (Language::Cpp, ProjectType::Executable) => "cxx_executable",
        (Language::C, ProjectType::Library) => "static_library",
        _ => "executable",
    };
    format!(
        "project \"{name}\" {{\n    description = \"A project built with Nox.\"\n\n    {target} \"{name}\" {{\n        sources = [\"{relative}\"]\n        install = true\n    }}\n}}\n"
    )
}

fn flake(language: Language, formatter_enabled: bool) -> String {
    let package = match language {
        Language::Rust => "rustc cargo rustfmt",
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

fn formatter(language: Language) -> Option<(&'static str, &'static str)> {
    match language {
        Language::Rust => Some(("rustfmt.toml", "edition = \"2024\"\n")),
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

fn nix_formatter(language: Language) -> String {
    let package = match language {
        Language::Rust => "rustfmt",
        Language::C | Language::Cpp => "clang-tools",
        Language::JavaScript | Language::TypeScript => "nodePackages.prettier",
        Language::Swift => "swift-format",
        _ => "nixfmt-rfc-style",
    };
    format!(
        "{{ pkgs }}: pkgs.writeShellApplication {{\n  name = \"project-format\";\n  runtimeInputs = [ pkgs.{package} ];\n  text = \"true\";\n}}\n"
    )
}

fn noxfile(language: Language) -> String {
    let format_command = match language {
        Language::Rust => "cargo fmt --all",
        Language::C | Language::Cpp => "clang-format -i $(find src -type f)",
        Language::JavaScript | Language::TypeScript => "npx prettier --write .",
        Language::Swift => "swift-format format --in-place Sources/*.swift",
        _ => "true",
    };
    format!(
        "tasks:\n    format:\n        run: {format_command}\n    check:\n        run: nox validate\n    build:\n        @nox check\n        run: nox build\n    test:\n        @nox build\n        run: true\n"
    )
}

fn swift_manifest(name: &str, project_type: ProjectType) -> String {
    let product = if project_type == ProjectType::Library {
        format!("products: [.library(name: \"{name}\", targets: [\"{name}\"])],")
    } else {
        String::new()
    };
    format!(
        "// swift-tools-version: 6.3\nimport PackageDescription\nlet package = Package(name: \"{name}\", {product} targets: [.executableTarget(name: \"{name}\", path: \"Sources\")])\n"
    )
}

fn readme(name: &str, nix: bool) -> String {
    let nix_line = if nix {
        "\nWith Nix installed, enter the development environment with `nix develop`.\n"
    } else {
        ""
    };
    format!(
        "# {name}\n\nA project built with Nox.\n\n## Build\n\n```sh\nnox setup\nnox build\n```\n{nix_line}"
    )
}

fn gitignore(language: Language) -> &'static str {
    match language {
        Language::Rust => "target/\nbuild/\n",
        Language::JavaScript | Language::TypeScript => "node_modules/\ndist/\n",
        _ => "build/\n",
    }
}

fn write_if_absent(path: &Path, contents: &str) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    output::path_value("generated", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Options, run};
    use std::fs;

    #[test]
    fn preserves_existing_nox_configuration() {
        let root = std::env::temp_dir().join(format!("nox-init-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join("nox.build"), "existing\n").unwrap();
        run(
            &root,
            Options {
                project_name: Some("fixture".to_string()),
                language: Some("rust".to_string()),
                project_type: Some("executable".to_string()),
                nix: false,
                noxfile: false,
                ..Options::default()
            },
        )
        .unwrap();
        assert_eq!(
            fs::read_to_string(root.join("nox.build")).unwrap(),
            "existing\n"
        );
        assert!(root.join("README.md").is_file());
        fs::remove_dir_all(root).unwrap();
    }
}
