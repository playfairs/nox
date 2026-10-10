use crate::core::error::{Error, Result};
use crate::core::output;
use crate::toolchain::rider::{Rider, RiderKind, available};
use std::io::{self, Write};
use std::process::{Command, ExitStatus};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PackageManager {
    Homebrew,
    Apt,
    Dnf,
    Pacman,
}

impl PackageManager {
    fn all() -> &'static [Self] {
        &[Self::Homebrew, Self::Apt, Self::Dnf, Self::Pacman]
    }

    fn executable(self) -> &'static str {
        match self {
            Self::Homebrew => "brew",
            Self::Apt => "apt-get",
            Self::Dnf => "dnf",
            Self::Pacman => "pacman",
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::Homebrew => "Homebrew",
            Self::Apt => "APT",
            Self::Dnf => "DNF",
            Self::Pacman => "Pacman",
        }
    }

    fn install_directory(self) -> Result<String> {
        match self {
            Self::Homebrew => {
                let output = Command::new("brew").arg("--prefix").output()?;
                if !output.status.success() {
                    return Err(process_error("brew --prefix", output.status));
                }
                let prefix = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if prefix.is_empty() {
                    return Err(Error::Config(
                        "Homebrew returned an empty install prefix".to_string(),
                    ));
                }
                Ok(format!("{prefix}/bin (Homebrew prefix: {prefix})"))
            }
            Self::Apt | Self::Dnf | Self::Pacman => {
                Ok("/usr (executables normally in /usr/bin)".to_string())
            }
        }
    }

    fn command(self, package: &str) -> (String, Vec<String>) {
        match self {
            Self::Homebrew => ("brew".to_string(), vec!["install".into(), package.into()]),
            Self::Apt => (
                "sudo".to_string(),
                vec![
                    "apt-get".into(),
                    "install".into(),
                    "-y".into(),
                    package.into(),
                ],
            ),
            Self::Dnf => (
                "sudo".to_string(),
                vec!["dnf".into(), "install".into(), "-y".into(), package.into()],
            ),
            Self::Pacman => (
                "sudo".to_string(),
                vec![
                    "pacman".into(),
                    "-S".into(),
                    "--needed".into(),
                    "--noconfirm".into(),
                    package.into(),
                ],
            ),
        }
    }

    fn package(self, tool: &str) -> Option<&'static str> {
        match (self, tool) {
            (Self::Homebrew, "gcc" | "g++") => Some("gcc"),
            (Self::Homebrew, "clang" | "clang++") => Some("llvm"),
            (Self::Homebrew, "rustc" | "cargo") => Some("rust"),
            (Self::Homebrew, "rustup") => Some("rustup"),
            (Self::Homebrew, "ldc2") => Some("ldc"),
            (Self::Homebrew, "dmd") => Some("dmd"),
            (Self::Homebrew, "ghc") => Some("ghc"),
            (Self::Homebrew, "go") => Some("go"),
            (Self::Homebrew, "javac") => Some("openjdk"),
            (Self::Homebrew, "dotnet") => Some("dotnet"),
            (Self::Homebrew, "mcs") => Some("mono"),
            (Self::Homebrew, "swiftc") => Some("swift"),
            (Self::Homebrew, "zig") => Some("zig"),
            (Self::Homebrew, "python3") => Some("python"),
            (Self::Homebrew, "node") => Some("node"),
            (Self::Homebrew, "tsc") => Some("typescript"),
            (Self::Homebrew, "kotlinc") => Some("kotlin"),
            (Self::Homebrew, "ruby") => Some("ruby"),
            (Self::Homebrew, "lua") => Some("lua"),
            (Self::Homebrew, "php") => Some("php"),
            (Self::Apt, "gcc" | "g++") => Some("build-essential"),
            (Self::Apt, "clang" | "clang++") => Some("clang"),
            (Self::Apt, "rustc") => Some("rustc"),
            (Self::Apt, "cargo") => Some("cargo"),
            (Self::Apt, "rustup") => Some("rustup"),
            (Self::Apt, "ldc2") => Some("ldc"),
            (Self::Apt, "dmd") => Some("dmd"),
            (Self::Apt, "gdc") => Some("gdc"),
            (Self::Apt, "ghc") => Some("ghc"),
            (Self::Apt, "go") => Some("golang"),
            (Self::Apt, "javac") => Some("default-jdk"),
            (Self::Apt, "dotnet") => Some("dotnet-sdk-8.0"),
            (Self::Apt, "mcs") => Some("mono-devel"),
            (Self::Apt, "python3") => Some("python3"),
            (Self::Apt, "node") => Some("nodejs"),
            (Self::Apt, "tsc") => Some("node-typescript"),
            (Self::Apt, "kotlinc") => Some("kotlin"),
            (Self::Apt, "ruby") => Some("ruby"),
            (Self::Apt, "lua") => Some("lua5.4"),
            (Self::Apt, "php") => Some("php-cli"),
            (Self::Dnf, "gcc") => Some("gcc"),
            (Self::Dnf, "g++") => Some("gcc-c++"),
            (Self::Dnf, "clang" | "clang++") => Some("clang"),
            (Self::Dnf, "rustc" | "cargo") => Some("rust"),
            (Self::Dnf, "rustup") => Some("rustup"),
            (Self::Dnf, "ldc2") => Some("ldc"),
            (Self::Dnf, "dmd") => Some("dmd"),
            (Self::Dnf, "gdc") => Some("gcc-gdc"),
            (Self::Dnf, "ghc") => Some("ghc"),
            (Self::Dnf, "go") => Some("golang"),
            (Self::Dnf, "javac") => Some("java-latest-openjdk-devel"),
            (Self::Dnf, "dotnet") => Some("dotnet-sdk-8.0"),
            (Self::Dnf, "mcs") => Some("mono-devel"),
            (Self::Dnf, "python3") => Some("python3"),
            (Self::Dnf, "node") => Some("nodejs"),
            (Self::Dnf, "tsc") => Some("nodejs-typescript"),
            (Self::Dnf, "kotlinc") => Some("kotlin"),
            (Self::Dnf, "ruby") => Some("ruby"),
            (Self::Dnf, "lua") => Some("lua"),
            (Self::Dnf, "php") => Some("php-cli"),
            (Self::Pacman, "gcc" | "g++") => Some("gcc"),
            (Self::Pacman, "clang" | "clang++") => Some("clang"),
            (Self::Pacman, "rustc" | "cargo") => Some("rust"),
            (Self::Pacman, "rustup") => Some("rustup"),
            (Self::Pacman, "ldc2") => Some("ldc"),
            (Self::Pacman, "dmd") => Some("dmd"),
            (Self::Pacman, "gdc") => Some("gdc"),
            (Self::Pacman, "ghc") => Some("ghc"),
            (Self::Pacman, "go") => Some("go"),
            (Self::Pacman, "javac") => Some("jdk-openjdk"),
            (Self::Pacman, "dotnet") => Some("dotnet-sdk"),
            (Self::Pacman, "mcs") => Some("mono"),
            (Self::Pacman, "swiftc") => Some("swift"),
            (Self::Pacman, "zig") => Some("zig"),
            (Self::Pacman, "python3") => Some("python"),
            (Self::Pacman, "node") => Some("nodejs"),
            (Self::Pacman, "tsc") => Some("typescript"),
            (Self::Pacman, "kotlinc") => Some("kotlin"),
            (Self::Pacman, "ruby") => Some("ruby"),
            (Self::Pacman, "lua") => Some("lua"),
            (Self::Pacman, "php") => Some("php"),
            _ => None,
        }
    }
}

#[derive(Clone, Copy)]
struct ToolChoice {
    tool: &'static str,
    description: &'static str,
}

fn choices(kind: RiderKind) -> &'static [ToolChoice] {
    match kind {
        RiderKind::C | RiderKind::Cpp => &[
            ToolChoice {
                tool: "gcc",
                description: "GCC (GNU compiler)",
            },
            ToolChoice {
                tool: "clang",
                description: "Clang/LLVM compiler",
            },
        ],
        RiderKind::Rust => &[
            ToolChoice {
                tool: "rustc",
                description: "rustc (Rust compiler)",
            },
            ToolChoice {
                tool: "cargo",
                description: "Cargo (build and dependency manager)",
            },
            ToolChoice {
                tool: "rustup",
                description: "rustup (Rust toolchain manager)",
            },
        ],
        RiderKind::D => &[
            ToolChoice {
                tool: "ldc2",
                description: "LDC (LLVM-based D compiler)",
            },
            ToolChoice {
                tool: "dmd",
                description: "DMD (reference D compiler)",
            },
            ToolChoice {
                tool: "gdc",
                description: "GDC (GNU D compiler)",
            },
        ],
        RiderKind::CSharp => &[
            ToolChoice {
                tool: "dotnet",
                description: ".NET SDK",
            },
            ToolChoice {
                tool: "mcs",
                description: "Mono C# compiler",
            },
        ],
        RiderKind::Java => &[ToolChoice {
            tool: "javac",
            description: "OpenJDK (Java compiler and runtime)",
        }],
        RiderKind::QSharp | RiderKind::FSharp => &[ToolChoice {
            tool: "dotnet",
            description: ".NET SDK",
        }],
        RiderKind::Haskell => &[ToolChoice {
            tool: "ghc",
            description: "GHC (Glasgow Haskell Compiler)",
        }],
        RiderKind::Go => &[ToolChoice {
            tool: "go",
            description: "Go toolchain",
        }],
        RiderKind::Swift => &[ToolChoice {
            tool: "swiftc",
            description: "Swift compiler",
        }],
        RiderKind::Zig => &[ToolChoice {
            tool: "zig",
            description: "Zig compiler",
        }],
        RiderKind::Python => &[ToolChoice {
            tool: "python3",
            description: "Python 3",
        }],
        RiderKind::JavaScript => &[ToolChoice {
            tool: "node",
            description: "Node.js",
        }],
        RiderKind::TypeScript => &[ToolChoice {
            tool: "tsc",
            description: "TypeScript compiler",
        }],
        RiderKind::Kotlin => &[ToolChoice {
            tool: "kotlinc",
            description: "Kotlin compiler",
        }],
        RiderKind::Ruby => &[ToolChoice {
            tool: "ruby",
            description: "Ruby",
        }],
        RiderKind::Lua => &[ToolChoice {
            tool: "lua",
            description: "Lua interpreter",
        }],
        RiderKind::Php => &[ToolChoice {
            tool: "php",
            description: "PHP runtime",
        }],
    }
}

struct InstallPlan {
    choice: ToolChoice,
    manager: PackageManager,
    package: &'static str,
}

pub fn run(arguments: &[String]) -> Result<()> {
    if arguments
        .first()
        .is_none_or(|argument| argument != "install")
    {
        return Err(Error::Config(
            "usage: nox riders [install <language>]".to_string(),
        ));
    }
    if arguments.len() != 2 {
        return Err(Error::Config(
            "usage: nox riders install <language>".to_string(),
        ));
    }

    let rider = find_rider(&arguments[1]).ok_or_else(|| {
        Error::Config(format!(
            "unknown Rider '{}'; run 'nox riders' to list supported languages",
            arguments[1]
        ))
    })?;
    if nix_is_available() {
        return recommend_nix(rider);
    }

    let managers: Vec<_> = PackageManager::all()
        .iter()
        .copied()
        .filter(|manager| command_available(manager.executable()))
        .collect();
    if managers.is_empty() {
        return Err(Error::Config(
            "no supported package manager found (supported: Homebrew, APT, DNF, Pacman)"
                .to_string(),
        ));
    }

    let mut plans = Vec::new();
    for choice in choices(rider.kind) {
        for manager in &managers {
            if let Some(package) = manager.package(choice.tool) {
                let (program, _) = manager.command(package);
                if program != manager.executable() && !command_available(&program) {
                    continue;
                }
                plans.push(InstallPlan {
                    choice: *choice,
                    manager: *manager,
                    package,
                });
            }
        }
    }
    if plans.is_empty() {
        return Err(Error::Config(format!(
            "no supported package-manager package is available to install {} on this system",
            rider.name
        )));
    }

    let selected = if plans.len() == 1 {
        0
    } else {
        output::section(format!("Install choices for {}", rider.name));
        for (index, plan) in plans.iter().enumerate() {
            println!(
                "{}. {} via {} (package: {})",
                index + 1,
                plan.choice.description,
                plan.manager.display_name(),
                plan.package
            );
        }
        choose("Select a toolchain option", plans.len())?
    };
    let plan = &plans[selected];
    install(plan)
}

fn nix_is_available() -> bool {
    command_available("nix")
        || std::env::var_os("IN_NIX_SHELL").is_some()
        || std::env::var_os("NIX_PROFILES").is_some()
        || std::env::var_os("NIX_PATH").is_some()
        || std::env::current_exe().is_ok_and(|path| {
            path.starts_with("/nix/store")
                || path.to_string_lossy().contains(".nix-profile/bin/nox")
        })
}

fn recommend_nix(rider: &Rider) -> Result<()> {
    let recommendations = choices(rider.kind)
        .iter()
        .filter_map(|choice| {
            nix_package(choice.tool).map(|package| {
                (
                    choice.description,
                    format!("nix profile add nixpkgs#{package}"),
                )
            })
        })
        .collect::<Vec<_>>();
    if recommendations.is_empty() {
        return Err(Error::Config(format!(
            "no supported nixpkgs toolchain packages are available for {}",
            rider.name
        )));
    }

    output::section(format!("Nix installation suggestions for {}", rider.name));
    output::key_value("Install location", "the active Nix user profile");
    for (tool, command) in recommendations {
        output::list_item(tool, command);
    }
    output::item("Nix is present or Nox is Nix-managed; Nox will not run another package manager.");
    output::item(
        "Only Nix-supported options are listed. Run the command(s) yourself; Nox will not install anything.",
    );
    Ok(())
}

fn nix_package(tool: &str) -> Option<&'static str> {
    match tool {
        "gcc" | "g++" => Some("gcc"),
        "clang" | "clang++" => Some("clang"),
        "rustc" => Some("rustc"),
        "cargo" => Some("cargo"),
        "rustup" => Some("rustup"),
        "ldc2" => Some("ldc"),
        "ghc" => Some("ghc"),
        "go" => Some("go"),
        "javac" => Some("jdk"),
        "dotnet" => Some("dotnet-sdk"),
        "mcs" => Some("mono"),
        "swiftc" => Some("swift"),
        "zig" => Some("zig"),
        "python3" => Some("python3"),
        "node" => Some("nodejs"),
        "tsc" => Some("typescript"),
        "kotlinc" => Some("kotlin"),
        "ruby" => Some("ruby"),
        "lua" => Some("lua"),
        "php" => Some("php"),
        _ => None,
    }
}

fn find_rider(name: &str) -> Option<&'static Rider> {
    let name = name.trim().to_ascii_lowercase();
    available().iter().find(|rider| {
        let rider_name = rider
            .name
            .strip_suffix(" Rider")
            .unwrap_or(rider.name)
            .to_ascii_lowercase();
        rider_name == name
            || match name.as_str() {
                "cpp" => rider.kind == RiderKind::Cpp,
                "js" => rider.kind == RiderKind::JavaScript,
                "ts" => rider.kind == RiderKind::TypeScript,
                "csharp" | "c#" => rider.kind == RiderKind::CSharp,
                "fsharp" | "f#" => rider.kind == RiderKind::FSharp,
                "php" => rider.kind == RiderKind::Php,
                _ => false,
            }
    })
}

fn install(plan: &InstallPlan) -> Result<()> {
    let install_directory = plan.manager.install_directory()?;
    let (program, arguments) = plan.manager.command(plan.package);
    let command_display = std::iter::once(program.as_str())
        .chain(arguments.iter().map(String::as_str))
        .map(shell_quote)
        .collect::<Vec<_>>()
        .join(" ");

    output::section("Installation preview");
    output::key_value("Tool", plan.choice.description);
    output::key_value("Package", plan.package);
    output::key_value("Package manager", plan.manager.display_name());
    output::key_value("Install location", install_directory);
    output::key_value("Command", &command_display);
    output::item("The package manager may also install required dependencies.");
    print!("Proceed with installation? [Y/n] ");
    io::stdout().flush()?;
    if !confirm()? {
        output::warning("installation cancelled");
        return Ok(());
    }

    output::action("installing", plan.choice.description);
    let mut command = Command::new(&program);
    command.args(&arguments);
    if plan.manager == PackageManager::Homebrew {
        command.env("HOMEBREW_NO_AUTO_UPDATE", "1");
    }
    let status = command.status()?;
    if !status.success() {
        return Err(process_error(&command_display, status));
    }
    output::success("Rider toolchain installed successfully.");
    Ok(())
}

fn choose(prompt: &str, option_count: usize) -> Result<usize> {
    print!("{prompt} [1-{option_count}]: ");
    io::stdout().flush()?;
    let selection = parse_selection(&read_prompt()?, option_count)?;
    Ok(selection)
}

fn parse_selection(input: &str, option_count: usize) -> Result<usize> {
    let selection = input
        .trim()
        .parse::<usize>()
        .map_err(|_| Error::Config("enter the number of the desired option".to_string()))?;
    if selection == 0 || selection > option_count {
        return Err(Error::Config(format!(
            "selection must be between 1 and {option_count}"
        )));
    }
    Ok(selection - 1)
}

fn read_prompt() -> Result<String> {
    let mut input = String::new();
    if io::stdin().read_line(&mut input)? == 0 {
        return Err(Error::Config(
            "no input received; installation cancelled".to_string(),
        ));
    }
    Ok(input)
}

fn confirm() -> Result<bool> {
    parse_confirmation(&read_prompt()?)
}

fn parse_confirmation(input: &str) -> Result<bool> {
    match input.trim().to_ascii_lowercase().as_str() {
        "" | "y" | "yes" => Ok(true),
        "n" | "no" => Ok(false),
        _ => Err(Error::Config("answer yes or no".to_string())),
    }
}

fn command_available(command: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|directory| {
            let path = directory.join(command);
            path.is_file() && {
                #[cfg(unix)]
                {
                    fs_permissions_executable(&path)
                }
                #[cfg(not(unix))]
                {
                    true
                }
            }
        })
    })
}

#[cfg(unix)]
fn fs_permissions_executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn fs_permissions_executable(_path: &std::path::Path) -> bool {
    true
}

fn shell_quote(argument: &str) -> String {
    if argument
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || "-_./".contains(character))
    {
        argument.to_string()
    } else {
        format!("'{}'", argument.replace('\'', "'\\''"))
    }
}

fn process_error(command: &str, status: ExitStatus) -> Error {
    Error::Process(format!("command '{command}' failed with status {status}"))
}

#[cfg(test)]
mod tests {
    use super::{
        PackageManager, choices, find_rider, nix_package, parse_confirmation, parse_selection,
        shell_quote,
    };
    use crate::toolchain::rider::RiderKind;

    #[test]
    fn resolves_rider_names_and_aliases() {
        assert_eq!(find_rider("rust").unwrap().kind, RiderKind::Rust);
        assert_eq!(find_rider("cpp").unwrap().kind, RiderKind::Cpp);
        assert_eq!(find_rider("ruby").unwrap().kind, RiderKind::Ruby);
        assert_eq!(find_rider("f#").unwrap().kind, RiderKind::FSharp);
    }

    #[test]
    fn rust_choices_map_to_installable_packages() {
        assert_eq!(PackageManager::Apt.package("rustc"), Some("rustc"));
        assert_eq!(PackageManager::Apt.package("cargo"), Some("cargo"));
        assert_eq!(PackageManager::Apt.package("rustup"), Some("rustup"));
        assert_eq!(PackageManager::Homebrew.package("rustc"), Some("rust"));
        assert_eq!(PackageManager::Apt.package("lua"), Some("lua5.4"));
        assert_eq!(PackageManager::Homebrew.package("php"), Some("php"));
    }

    #[test]
    fn nix_suggestions_use_nixpkgs_attributes() {
        assert_eq!(nix_package("rustc"), Some("rustc"));
        assert_eq!(nix_package("cargo"), Some("cargo"));
        assert_eq!(nix_package("rustup"), Some("rustup"));
        assert_eq!(nix_package("tsc"), Some("typescript"));
        assert_eq!(nix_package("not-a-tool"), None);
        assert_eq!(nix_package("dmd"), None);
        assert_eq!(nix_package("gdc"), None);
    }

    #[test]
    fn rust_nix_suggestions_cover_each_tool_without_interactive_selection() {
        let suggestions: Vec<_> = choices(RiderKind::Rust)
            .iter()
            .map(|choice| {
                format!(
                    "nix profile add nixpkgs#{}",
                    nix_package(choice.tool).unwrap()
                )
            })
            .collect();
        assert_eq!(
            suggestions,
            [
                "nix profile add nixpkgs#rustc",
                "nix profile add nixpkgs#cargo",
                "nix profile add nixpkgs#rustup",
            ]
        );
    }

    #[test]
    fn nix_suggestions_omit_unavailable_d_compilers() {
        let suggestions: Vec<_> = choices(RiderKind::D)
            .iter()
            .filter_map(|choice| {
                nix_package(choice.tool).map(|package| format!("nix profile add nixpkgs#{package}"))
            })
            .collect();
        assert_eq!(suggestions, ["nix profile add nixpkgs#ldc"]);
    }

    #[test]
    fn quotes_preview_arguments_that_need_it() {
        assert_eq!(shell_quote("apt-get"), "apt-get");
        assert_eq!(shell_quote("two words"), "'two words'");
        assert_eq!(shell_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn selection_is_bounded_to_the_displayed_options() {
        assert_eq!(parse_selection("2\n", 3).unwrap(), 1);
        assert!(parse_selection("0", 3).is_err());
        assert!(parse_selection("4", 3).is_err());
    }

    #[test]
    fn confirmation_defaults_to_yes_and_accepts_denial() {
        assert!(parse_confirmation("\n").unwrap());
        assert!(parse_confirmation("Y\n").unwrap());
        assert!(!parse_confirmation("n\n").unwrap());
        assert!(parse_confirmation("maybe").is_err());
    }
}
