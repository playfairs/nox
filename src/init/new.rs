use super::buildgen;
use super::config::Options;
use super::error::Result;
use super::language::{Language, ProjectType};
use super::project::ProjectInfo;
use super::templates;
use std::fs;
use std::path::Path;

pub fn create_files(
    root: &Path,
    project: &ProjectInfo,
    language: Language,
    project_type: ProjectType,
    options: &Options,
    name: &str,
) -> Result<()> {
    if project
        .source_files
        .iter()
        .all(|path| super::language::is_test_path(path))
    {
        let (path, contents) = templates::starter(language, project_type);
        write_if_absent(&root.join(path), contents)?;
    }
    if language == Language::Rust && !root.join("Cargo.toml").exists() {
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
            &templates::swift_manifest(name, project_type),
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
        &buildgen::render(project, language, project_type, name),
    )?;
    if options.noxfile && !project.has_noxfile {
        write_if_absent(&root.join("noxfile"), &templates::noxfile(language))?;
    }
    if options.nix && !project.has_flake {
        write_if_absent(
            &root.join("flake.nix"),
            &templates::flake(language, options.formatter),
        )?;
    }
    if options.formatter && project.formatter_configs.is_empty() {
        if let Some((path, contents)) = templates::formatter(language) {
            write_if_absent(&root.join(path), contents)?;
        }
    }
    if options.nix && options.formatter && !project.has_nix_formatter {
        fs::create_dir_all(root.join("nix"))?;
        write_if_absent(
            &root.join("nix/formatter.nix"),
            &templates::nix_formatter(language),
        )?;
    }
    if !project.has_readme {
        write_if_absent(
            &root.join("README.md"),
            &templates::readme(name, options.nix),
        )?;
    }
    if !root.join(".gitignore").exists() {
        write_if_absent(&root.join(".gitignore"), templates::gitignore(language))?;
    }
    Ok(())
}
fn write_if_absent(path: &Path, contents: &str) -> Result<()> {
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    crate::core::output::path_value("generated", path.display());
    Ok(())
}
