use crate::{parse, serialize, Value};
use std::fs;
use std::path::Path;

pub fn format_text(input: &str) -> Result<String, String> {
    let value = parse(input).map_err(|error| error.to_string())?;
    Ok(serialize(&value))
}

pub fn format_file(path: impl AsRef<Path>) -> Result<String, String> {
    let text = fs::read_to_string(path.as_ref())
        .map_err(|error| format!("failed to read {}: {error}", path.as_ref().display()))?;
    format_text(&text)
}

pub fn format_file_in_place(path: impl AsRef<Path>) -> Result<(), String> {
    let path = path.as_ref();
    let text = fs::read_to_string(path)
        .map_err(|error| format!("failed to read {}: {error}", path.display()))?;
    let formatted = format_text(&text)?;
    fs::write(path, formatted)
        .map_err(|error| format!("failed to write {}: {error}", path.display()))?;
    Ok(())
}

pub fn format_value(value: &Value) -> String {
    serialize(value)
}
