use super::buildgen;
use super::config::Options;
use super::error::Result;
use super::language::{Language, ProjectType};
use super::project::ProjectInfo;
use super::templates;
use crate::rules::init::InitRules;
use std::fs;
use std::path::Path;

pub fn create_files(
    root: &Path,
    project: &ProjectInfo,
    rules: &InitRules,
    language: Language,
    project_type: ProjectType,
    options: &Options,
    name: &str,
) -> Result<()> {
    if project
        .source_files
        .iter()
        .all(|path| super::language::is_test_path_with_rules(path, rules))
    {
        let (path, contents) = templates::starter(language, project_type);
        write_if_absent(&root.join(path), contents)?;
    }
    if rules.template("rust_manifest", language).is_some() && language == Language::Rust && !root.join("Cargo.toml").exists() {
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
    if rules.template("javascript_manifest", language).is_some()
        && matches!(language, Language::JavaScript | Language::TypeScript)
        && !root.join("package.json").exists()
    {
        write_if_absent(
            &root.join("package.json"),
            &format!(
                "{{\n  \"name\": \"{name}\",\n  \"version\": \"0.1.0\",\n  \"private\": true\n}}\n"
            ),
        )?;
    }
    if rules.template("typescript_config", language).is_some() && language == Language::TypeScript && !root.join("tsconfig.json").exists() {
        write_if_absent(
            &root.join("tsconfig.json"),
            "{\n  \"compilerOptions\": {\n    \"target\": \"ES2022\",\n    \"module\": \"commonjs\",\n    \"outDir\": \"dist\",\n    \"strict\": true\n  },\n  \"include\": [\"src\"]\n}\n",
        )?;
    }
    if rules.template("swift_manifest", language).is_some() && language == Language::Swift && !root.join("Package.swift").exists() {
        write_if_absent(
            &root.join("Package.swift"),
            &templates::swift_manifest(name, project_type),
        )?;
    }
    if rules.template("python_manifest", language).is_some() && language == Language::Python && !root.join("pyproject.toml").exists() {
        write_if_absent(
            &root.join("pyproject.toml"),
            &format!(
                "[project]\nname = \"{name}\"\nversion = \"0.1.0\"\nrequires-python = \">=3.10\"\n"
            ),
        )?;
    }
    write_if_absent(
        &root.join("nox.build"),
        &buildgen::render(project, rules, language, project_type, name),
    )?;
    if options.noxfile && rules.template("noxfile", language).is_some() && !project.has_noxfile {
        write_if_absent(&root.join("noxfile"), &templates::noxfile(language))?;
    }
    if options.nix && rules.template("flake", language).is_some() && !project.has_flake {
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
    if rules.template("readme", language).is_some() && !project.has_readme {
        write_if_absent(
            &root.join("README.md"),
            &templates::readme(name, options.nix),
        )?;
    }
    if rules.template("gitignore", language).is_some() && !root.join(".gitignore").exists() {
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
