use askalono::{Store, TextData};
use license::License;
use std::fs;
use std::path::Path;

const SPDX_CANDIDATES: &[&str] = &[
    "0BSD",
    "AGPL-3.0-only",
    "Apache-2.0",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "CC0-1.0",
    "GPL-2.0-only",
    "GPL-3.0-only",
    "ISC",
    "LGPL-2.1-only",
    "MIT",
    "MPL-2.0",
    "Unlicense",
    "Zlib",
];

pub fn identify_file(path: &Path) -> Result<String, String> {
    let contents = fs::read_to_string(path)
        .map_err(|error| format!("could not read license file '{}': {error}", path.display()))?;
    identify_text(&contents).ok_or_else(|| {
        format!(
            "could not identify a recognized SPDX license in '{}'",
            path.display()
        )
    })
}

fn identify_text(contents: &str) -> Option<String> {
    let mut store = Store::new();
    for identifier in SPDX_CANDIDATES {
        if let Ok(license) = identifier.parse::<&dyn License>() {
            store.add_license((*identifier).to_string(), TextData::from(license.text()));
        }
    }
    let matched = store.analyze(&TextData::from(contents));
    (matched.score >= 0.85).then(|| matched.name.to_string())
}

#[cfg(test)]
mod tests {
    use super::identify_text;

    #[test]
    fn identifies_unlicense_text() {
        let text = include_str!("../../LICENSE");
        assert_eq!(identify_text(text).as_deref(), Some("Unlicense"));
    }

    #[test]
    fn rejects_unrecognized_license_text() {
        assert!(identify_text("This is not a license.").is_none());
    }
}
