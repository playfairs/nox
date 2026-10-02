use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RiderKind {
    C,
    Cpp,
    Rust,
    Haskell,
    D,
    Go,
    Java,
    Kotlin,
    FSharp,
    CSharp,
    QSharp,
    Swift,
    Zig,
    Python,
    JavaScript,
    TypeScript,
    Ruby,
    Lua,
    Php,
}

#[derive(Clone, Copy, Debug)]
pub struct Rider {
    pub name: &'static str,
    pub kind: RiderKind,
    pub description: &'static str,
    pub extensions: &'static [&'static str],
}

pub fn available() -> &'static [Rider] {
    &[
        Rider {
            name: "C Rider",
            kind: RiderKind::C,
            description: "Builds C executables, static libraries, and shared libraries.",
            extensions: &["c"],
        },
        Rider {
            name: "C++ Rider",
            kind: RiderKind::Cpp,
            description: "Builds C++ executables, static libraries, and shared libraries.",
            extensions: &["cpp", "cc", "cxx"],
        },
        Rider {
            name: "Rust Rider",
            kind: RiderKind::Rust,
            description: "Builds Rust executables and libraries through rustc and Cargo.",
            extensions: &["rs"],
        },
        Rider {
            name: "Haskell Rider",
            kind: RiderKind::Haskell,
            description: "Builds Haskell executables through GHC.",
            extensions: &["hs", "lhs"],
        },
        Rider {
            name: "D Rider",
            kind: RiderKind::D,
            description: "Builds D executables through ldc2, dmd, or gdc.",
            extensions: &["d"],
        },
        Rider {
            name: "Go Rider",
            kind: RiderKind::Go,
            description: "Builds Go executables through the Go toolchain.",
            extensions: &["go"],
        },
        Rider {
            name: "Java Rider",
            kind: RiderKind::Java,
            description: "Builds Java archives through javac and jar.",
            extensions: &["java"],
        },
        Rider {
            name: "C# Rider",
            kind: RiderKind::CSharp,
            description: "Builds C# executables through csc or mcs.",
            extensions: &["cs"],
        },
        Rider {
            name: "Q# Rider",
            kind: RiderKind::QSharp,
            description: "Builds Q# projects through the .NET SDK and Microsoft.Quantum.Sdk.",
            extensions: &["qs"],
        },
        Rider {
            name: "Swift Rider",
            kind: RiderKind::Swift,
            description: "Builds Swift executables through swiftc.",
            extensions: &["swift"],
        },
        Rider {
            name: "Zig Rider",
            kind: RiderKind::Zig,
            description: "Builds Zig executables through zig.",
            extensions: &["zig"],
        },
        Rider {
            name: "Python Rider",
            kind: RiderKind::Python,
            description: "Checks and packages Python scripts through Python.",
            extensions: &["py"],
        },
        Rider {
            name: "JavaScript Rider",
            kind: RiderKind::JavaScript,
            description: "Checks and packages JavaScript through Node.js.",
            extensions: &["js", "jsx", "mjs"],
        },
        Rider {
            name: "TypeScript Rider",
            kind: RiderKind::TypeScript,
            description: "Transpiles TypeScript through tsc.",
            extensions: &["ts", "tsx"],
        },
        Rider {
            name: "Kotlin Rider",
            kind: RiderKind::Kotlin,
            description: "Builds Kotlin archives through kotlinc.",
            extensions: &["kt", "kts"],
        },
        Rider {
            name: "Ruby Rider",
            kind: RiderKind::Ruby,
            description: "Runs Ruby scripts through Ruby.",
            extensions: &["rb"],
        },
        Rider {
            name: "F# Rider",
            kind: RiderKind::FSharp,
            description: "Runs F# scripts through .NET F# Interactive.",
            extensions: &["fsx"],
        },
        Rider {
            name: "Lua Rider",
            kind: RiderKind::Lua,
            description: "Runs Lua scripts through the Lua interpreter.",
            extensions: &["lua"],
        },
        Rider {
            name: "PHP Rider",
            kind: RiderKind::Php,
            description: "Runs PHP scripts through the PHP runtime.",
            extensions: &["php"],
        },
    ]
}

pub fn for_source(source: &Path) -> Option<&'static Rider> {
    let extension = source.extension()?.to_str()?;
    available()
        .iter()
        .find(|rider| rider.extensions.contains(&extension))
}

#[cfg(test)]
mod tests {
    use super::{RiderKind, available, for_source};
    use std::path::Path;

    #[test]
    fn resolves_haskell_sources() {
        assert_eq!(
            for_source(Path::new("Main.hs")).unwrap().kind,
            RiderKind::Haskell
        );
        assert_eq!(
            for_source(Path::new("Main.lhs")).unwrap().kind,
            RiderKind::Haskell
        );
    }

    #[test]
    fn resolves_qsharp_sources() {
        assert_eq!(
            for_source(Path::new("main.qs")).unwrap().kind,
            RiderKind::QSharp
        );
    }

    #[test]
    fn resolves_ruby_and_fsharp_sources() {
        assert_eq!(
            for_source(Path::new("script.rb")).unwrap().kind,
            RiderKind::Ruby
        );
        assert_eq!(
            for_source(Path::new("script.fsx")).unwrap().kind,
            RiderKind::FSharp
        );
    }

    #[test]
    fn resolves_lua_and_php_sources() {
        assert_eq!(
            for_source(Path::new("script.lua")).unwrap().kind,
            RiderKind::Lua
        );
        assert_eq!(
            for_source(Path::new("script.php")).unwrap().kind,
            RiderKind::Php
        );
    }

    #[test]
    fn rider_names_are_unique() {
        let mut names: Vec<_> = available().iter().map(|rider| rider.name).collect();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), available().len());
    }
}
