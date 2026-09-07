use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RiderKind {
    C,
    Cpp,
    Rust,
    Go,
    Java,
    CSharp,
    Swift,
    Zig,
    Python,
    JavaScript,
    TypeScript,
    Kotlin,
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
            description: "Builds basic Rust executables and libraries through rustc.",
            extensions: &["rs"],
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
            extensions: &["js", "jsx"],
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
    ]
}

pub fn for_source(source: &Path) -> Option<&'static Rider> {
    let extension = source.extension()?.to_str()?;
    available()
        .iter()
        .find(|rider| rider.extensions.contains(&extension))
}
