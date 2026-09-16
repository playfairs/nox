use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub type_name: String,
    pub name: String,
    pub properties: BTreeMap<String, Value>,
    pub extends: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ruleset {
    pub name: String,
    pub entries: Vec<Entry>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Object(BTreeMap<String, Value>),
    Array(Vec<Value>),
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Null,
    Ruleset(Ruleset),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ParseError {}

pub fn parse(input: &str) -> Result<Value, ParseError> {
    let mut parser = Parser::new(input);
    let value = parser.parse_value()?;
    parser.skip_ws_comments();
    if !parser.is_eof() {
        return Err(ParseError {
            message: format!("unexpected trailing input: '{}'", parser.rest()),
        });
    }
    resolve_extensions(value, None, &mut Vec::new())
}

pub fn parse_file(path: &str) -> Result<Value, ParseError> {
    let text = std::fs::read_to_string(path).map_err(|error| ParseError {
        message: format!("failed to read {}: {}", path, error),
    })?;
    parse(&text)
}

pub fn serialize(value: &Value) -> String {
    match value {
        Value::Object(entries) => {
            let mut out = String::from("{\n");
            let mut first = true;
            for (key, child) in entries {
                if !first {
                    out.push('\n');
                }
                first = false;
                out.push_str("    ");
                out.push_str(&format_key(key));
                out.push_str(": ");
                out.push_str(&serialize_value(child, 1));
            }
            out.push('\n');
            out.push('}');
            out
        }
        Value::Array(values) => {
            let mut out = String::from("[");
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                out.push_str(&serialize_array_element(value, 0));
            }
            out.push(']');
            out
        }
        Value::String(value) => format_scalar_string(value),
        Value::Integer(value) => value.to_string(),
        Value::Float(value) => {
            if value.fract() == 0.0 {
                format!("{}.0", value)
            } else {
                value.to_string()
            }
        }
        Value::Boolean(value) => value.to_string(),
        Value::Null => "null".to_string(),
        Value::Ruleset(ruleset) => serialize_ruleset(ruleset),
    }
}

fn serialize_ruleset(ruleset: &Ruleset) -> String {
    let mut out = String::new();
    out.push_str("ruleset ");
    out.push_str(&format_string_literal(&ruleset.name));
    out.push_str(" {\n");
    for (index, entry) in ruleset.entries.iter().enumerate() {
        if index > 0 {
            out.push_str(",\n");
        }
        out.push_str("    ");
        out.push_str(&entry.type_name);
        out.push_str(": ");
        out.push_str(&serialize_entry_name(&entry.name));
        if let Some(parent) = &entry.extends {
            out.push_str(" extends ");
            out.push_str(&serialize_entry_name(parent));
        }
        out.push_str(" {\n");

        let mut first = true;
        for (key, value) in &entry.properties {
            if key == "extends" {
                continue;
            }
            if !first {
                out.push('\n');
            }
            first = false;
            out.push_str("        ");
            out.push_str(&format_key(key));
            out.push_str(": ");
            out.push_str(&serialize_value(value, 8));
        }

        if !first {
            out.push('\n');
        }
        out.push_str("    }");
    }
    out.push_str("\n}");
    out
}

fn serialize_entry_name(value: &str) -> String {
    if looks_like_identifier(value) {
        value.to_string()
    } else {
        format_string_literal(value)
    }
}

fn serialize_value(value: &Value, indent: usize) -> String {
    match value {
        Value::Object(entries) => {
            let mut out = String::from("{\n");
            let mut first = true;
            for (key, child) in entries {
                if !first {
                    out.push('\n');
                }
                first = false;
                out.push_str(&" ".repeat(indent + 4));
                out.push_str(&format_key(key));
                out.push_str(": ");
                out.push_str(&serialize_value(child, indent + 4));
            }
            out.push('\n');
            out.push_str(&" ".repeat(indent));
            out.push('}');
            out
        }
        Value::Array(values) => {
            let mut out = String::from("[");
            for (index, child) in values.iter().enumerate() {
                if index > 0 {
                    out.push_str(", ");
                }
                out.push_str(&serialize_array_element(child, indent + 2));
            }
            out.push(']');
            out
        }
        Value::String(value) => format_scalar_string(value),
        Value::Integer(value) => value.to_string(),
        Value::Float(value) => {
            if value.fract() == 0.0 {
                format!("{}.0", value)
            } else {
                value.to_string()
            }
        }
        Value::Boolean(value) => value.to_string(),
        Value::Null => "null".to_string(),
        Value::Ruleset(ruleset) => serialize_ruleset(ruleset),
    }
}

fn serialize_array_element(value: &Value, indent: usize) -> String {
    match value {
        Value::String(value) => format_string_literal(value),
        _ => serialize_value(value, indent),
    }
}

fn format_key(key: &str) -> String {
    if looks_like_identifier(key) {
        key.to_string()
    } else {
        format!("\"{}\"", escape_string(key))
    }
}

fn format_scalar_string(value: &str) -> String {
    if looks_like_identifier(value) {
        value.to_string()
    } else {
        format_string_literal(value)
    }
}

fn format_string_literal(value: &str) -> String {
    format!("\"{}\"", escape_string(value))
}

fn looks_like_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-'))
        && value
            .chars()
            .next()
            .map_or(false, |c| c.is_ascii_alphabetic() || c == '_')
}

fn escape_string(value: &str) -> String {
    let mut out = String::new();
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out
}

fn resolve_extensions(
    value: Value,
    scope: Option<&BTreeMap<String, Value>>,
    stack: &mut Vec<String>,
) -> Result<Value, ParseError> {
    match value {
        Value::Object(map) => {
            let mut merged = BTreeMap::new();

            if let Some(extension_name) = map.get("extends").and_then(|value| match value {
                Value::String(name) => Some(name.clone()),
                _ => None,
            }) {
                if stack.iter().any(|name| name == &extension_name) {
                    return Err(ParseError {
                        message: format!("circular extension detected through '{extension_name}'"),
                    });
                }
                let parent = scope
                    .and_then(|scope| scope.get(&extension_name))
                    .cloned()
                    .ok_or_else(|| ParseError {
                        message: format!("missing extension target '{extension_name}'"),
                    })?;

                let parent = resolve_extensions(parent, scope, &mut {
                    let mut next = stack.clone();
                    next.push(extension_name.clone());
                    next
                })?;
                let Value::Object(parent_map) = parent else {
                    return Err(ParseError {
                        message: format!("extension target '{extension_name}' is not an object"),
                    });
                };
                for (key, value) in parent_map {
                    merged.insert(key, value);
                }
            }

            let current_scope = Some(map.clone());
            for (key, value) in map {
                if key == "extends" {
                    continue;
                }
                merged.insert(
                    key,
                    resolve_extensions(value, current_scope.as_ref().or(scope), stack)?,
                );
            }

            Ok(Value::Object(merged))
        }
        Value::Array(values) => Ok(Value::Array(
            values
                .into_iter()
                .map(|value| resolve_extensions(value, scope, stack))
                .collect::<Result<Vec<_>, _>>()?,
        )),
        Value::Ruleset(ruleset) => Ok(Value::Ruleset(resolve_ruleset_extensions(ruleset, stack)?)),
        other => Ok(other),
    }
}

fn resolve_ruleset_extensions(
    ruleset: Ruleset,
    stack: &mut Vec<String>,
) -> Result<Ruleset, ParseError> {
    let mut resolved = Vec::new();
    for entry in &ruleset.entries {
        resolved.push(resolve_entry_extensions(entry.clone(), &ruleset, stack)?);
    }
    Ok(Ruleset {
        name: ruleset.name,
        entries: resolved,
    })
}

fn resolve_entry_extensions(
    entry: Entry,
    ruleset: &Ruleset,
    stack: &mut Vec<String>,
) -> Result<Entry, ParseError> {
    let extension_name = entry
        .extends
        .clone()
        .or_else(|| match entry.properties.get("extends") {
            Some(Value::String(name)) => Some(name.clone()),
            _ => None,
        });

    let mut merged = entry.properties.clone();
    if let Some(extension_name) = extension_name {
        let key = format!("{}:{}", entry.type_name, entry.name);
        if stack
            .iter()
            .any(|name| name == &key || name == &extension_name)
        {
            return Err(ParseError {
                message: format!("circular extension detected through '{extension_name}'"),
            });
        }

        let parent = ruleset
            .entries
            .iter()
            .find(|candidate| {
                candidate.type_name == entry.type_name && candidate.name == extension_name
            })
            .cloned()
            .ok_or_else(|| ParseError {
                message: format!("missing extension target '{extension_name}'"),
            })?;

        let mut next_stack = stack.clone();
        next_stack.push(key.clone());
        let resolved_parent = resolve_entry_extensions(parent, ruleset, &mut next_stack)?;

        let mut parent_properties = resolved_parent.properties;
        parent_properties.remove("extends");
        for (key, value) in parent_properties {
            merged.entry(key).or_insert(value);
        }
        for (key, value) in entry.properties {
            if key != "extends" {
                merged.insert(key, value);
            }
        }
        merged.remove("extends");
    }

    Ok(Entry {
        type_name: entry.type_name,
        name: entry.name,
        properties: merged,
        extends: None,
    })
}

struct Parser {
    chars: Vec<char>,
    pos: usize,
}

impl Parser {
    fn new(input: &str) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
        }
    }

    fn parse_value(&mut self) -> Result<Value, ParseError> {
        self.skip_ws_comments();
        if self.check_keyword("ruleset") {
            return self.parse_ruleset();
        }
        if self.check('{') {
            return self.parse_object_block();
        }
        if self.check('[') {
            return self.parse_array();
        }
        if self.is_property_start() {
            return self.parse_object_entries_until(None);
        }
        self.parse_scalar()
    }

    fn parse_object_block(&mut self) -> Result<Value, ParseError> {
        self.expect('{')?;
        let value = self.parse_object_entries_until(Some('}'))?;
        self.expect('}')?;
        Ok(value)
    }

    fn parse_ruleset(&mut self) -> Result<Value, ParseError> {
        self.expect_keyword("ruleset")?;
        self.skip_ws_comments();
        let name = match self.parse_string()? {
            Value::String(name) => name,
            other => {
                return Err(ParseError {
                    message: format!("ruleset name must be a string, got {other:?}"),
                });
            }
        };
        self.skip_ws_comments();
        self.expect('{')?;
        let entries = self.parse_ruleset_entries_until(Some('}'))?;
        self.expect('}')?;
        Ok(Value::Ruleset(Ruleset { name, entries }))
    }

    fn parse_array(&mut self) -> Result<Value, ParseError> {
        self.expect('[')?;
        self.skip_ws_comments();
        if self.check(']') {
            self.pos += 1;
            return Ok(Value::Array(vec![]));
        }

        let mut values = Vec::new();
        loop {
            self.skip_ws_comments();
            if self.check(']') {
                self.pos += 1;
                break;
            }
            values.push(self.parse_value()?);
            self.skip_ws_comments();
            if self.check(',') {
                self.pos += 1;
                continue;
            }
            if self.check(']') {
                self.pos += 1;
                break;
            }
            return Err(ParseError {
                message: format!("expected ',' or ']' in array, found '{:?}'.", self.peek()),
            });
        }
        Ok(Value::Array(values))
    }

    fn parse_ruleset_entries_until(
        &mut self,
        terminator: Option<char>,
    ) -> Result<Vec<Entry>, ParseError> {
        let mut entries = Vec::new();
        loop {
            self.skip_ws_comments();
            if terminator.is_some_and(|ch| self.check(ch)) {
                break;
            }
            if self.is_eof() {
                break;
            }
            let type_name = self.parse_key()?;
            self.skip_ws_comments();
            self.expect(':')?;
            let name = self.parse_entry_name()?;
            self.skip_ws_comments();
            let mut extends = None;
            if self.check_keyword("extends") {
                self.expect_keyword("extends")?;
                self.skip_ws_comments();
                extends = Some(self.parse_entry_name()?);
                self.skip_ws_comments();
            }
            if !self.check('{') {
                return Err(ParseError {
                    message: format!("expected '{{' after entry '{type_name}: {name}'"),
                });
            }
            self.expect('{')?;
            let properties = self.parse_object_entries_until(Some('}'))?;
            self.expect('}')?;
            entries.push(Entry {
                type_name,
                name,
                properties: match properties {
                    Value::Object(inner) => inner,
                    _ => unreachable!(),
                },
                extends,
            });
            self.skip_ws_comments();
            if self.check(',') {
                self.pos += 1;
                continue;
            }
            if terminator.is_some_and(|ch| self.check(ch)) {
                break;
            }
            if self.is_eof() {
                break;
            }
            if self.check('}') {
                break;
            }
            if self.is_property_start() {
                continue;
            }
            break;
        }
        Ok(entries)
    }

    fn parse_object_entries_until(
        &mut self,
        terminator: Option<char>,
    ) -> Result<Value, ParseError> {
        let mut map = BTreeMap::new();
        loop {
            self.skip_ws_comments();
            if terminator.is_some_and(|ch| self.check(ch)) {
                break;
            }
            if self.is_eof() {
                break;
            }
            let key = self.parse_key()?;
            self.skip_ws_comments();
            self.expect(':')?;
            let value = self.parse_value()?;
            if map.contains_key(&key) {
                return Err(ParseError {
                    message: format!("duplicate property '{key}'"),
                });
            }
            map.insert(key, value);
            self.skip_ws_comments();
            if self.check(',') {
                self.pos += 1;
                continue;
            }
            if terminator.is_some_and(|ch| self.check(ch)) {
                break;
            }
            if self.is_property_start() {
                continue;
            }
            if self.is_eof() {
                break;
            }
            if self.check(']') || self.check('}') {
                break;
            }
            break;
        }
        Ok(Value::Object(map))
    }

    fn parse_key(&mut self) -> Result<String, ParseError> {
        self.skip_ws_comments();
        let start = self.pos;
        let Some(ch) = self.peek() else {
            return Err(ParseError {
                message: "expected property name".to_string(),
            });
        };
        if !(ch.is_ascii_alphabetic() || ch == '_') {
            return Err(ParseError {
                message: format!("invalid property name starting with '{}'", ch),
            });
        }
        self.pos += 1;
        while let Some(next) = self.peek() {
            if next.is_ascii_alphanumeric() || matches!(next, '_' | '-') {
                self.pos += 1;
            } else {
                break;
            }
        }
        Ok(self.chars[start..self.pos].iter().collect())
    }

    fn parse_entry_name(&mut self) -> Result<String, ParseError> {
        self.skip_ws_comments();
        if self.check('"') {
            return self.parse_string().map(|value| match value {
                Value::String(text) => text,
                _ => unreachable!(),
            });
        }
        self.parse_key()
    }

    fn parse_scalar(&mut self) -> Result<Value, ParseError> {
        self.skip_ws_comments();
        let start = self.pos;
        let Some(ch) = self.peek() else {
            return Err(ParseError {
                message: "unexpected end of input".to_string(),
            });
        };

        if ch == '"' {
            return self.parse_string();
        }

        if ch == '-' || ch.is_ascii_digit() {
            let mut saw_digit = ch.is_ascii_digit();
            let mut has_dot = false;
            let negative = ch == '-';
            self.pos += 1;
            while let Some(next) = self.peek() {
                if next.is_ascii_digit() {
                    saw_digit = true;
                    self.pos += 1;
                } else if next == '.' && !has_dot && !negative {
                    has_dot = true;
                    self.pos += 1;
                } else {
                    break;
                }
            }
            let text: String = self.chars[start..self.pos].iter().collect();
            if text == "-" {
                return Err(ParseError {
                    message: "invalid number".to_string(),
                });
            }
            if has_dot {
                let value = text.parse::<f64>().map_err(|_| ParseError {
                    message: format!("invalid float '{text}'"),
                })?;
                return Ok(Value::Float(value));
            }
            if saw_digit {
                let value = text.parse::<i64>().map_err(|_| ParseError {
                    message: format!("invalid integer '{text}'"),
                })?;
                return Ok(Value::Integer(value));
            }
        }

        let mut end = self.pos;
        while let Some(next) = self.peek_n_from(end) {
            if next.is_whitespace() || matches!(next, '[' | ']' | '{' | '}' | ',' | ':') {
                break;
            }
            end += 1;
        }
        let text: String = self.chars[self.pos..end].iter().collect();
        self.pos = end;
        match text.as_str() {
            "true" => Ok(Value::Boolean(true)),
            "false" => Ok(Value::Boolean(false)),
            "null" => Ok(Value::Null),
            _ => Ok(Value::String(text)),
        }
    }

    fn parse_string(&mut self) -> Result<Value, ParseError> {
        self.expect('"')?;
        let mut out = String::new();
        while let Some(ch) = self.peek() {
            if ch == '"' {
                self.pos += 1;
                return Ok(Value::String(out));
            }
            if ch == '\\' {
                self.pos += 1;
                let Some(next) = self.peek() else {
                    return Err(ParseError {
                        message: "unterminated escape sequence".to_string(),
                    });
                };
                let escaped = match next {
                    'n' => '\n',
                    'r' => '\r',
                    't' => '\t',
                    '\\' => '\\',
                    '"' => '"',
                    'u' => {
                        self.pos += 1;
                        let mut digits = String::new();
                        for _ in 0..4 {
                            match self.peek() {
                                Some(ch) if ch.is_ascii_hexdigit() => {
                                    digits.push(ch);
                                    self.pos += 1;
                                }
                                _ => {
                                    return Err(ParseError {
                                        message: "invalid unicode escape".to_string(),
                                    });
                                }
                            }
                        }
                        let codepoint =
                            u32::from_str_radix(&digits, 16).map_err(|_| ParseError {
                                message: "invalid unicode escape".to_string(),
                            })?;
                        let ch = char::from_u32(codepoint).ok_or(ParseError {
                            message: "invalid unicode escape".to_string(),
                        })?;
                        out.push(ch);
                        continue;
                    }
                    other => other,
                };
                out.push(escaped);
                self.pos += 1;
                continue;
            }
            out.push(ch);
            self.pos += 1;
        }
        Err(ParseError {
            message: "unterminated string".to_string(),
        })
    }

    fn is_property_start(&self) -> bool {
        let mut index = self.pos;
        while index < self.chars.len() && self.chars[index].is_whitespace() {
            index += 1;
        }
        if index >= self.chars.len() {
            return false;
        }
        if !self.chars[index].is_ascii_alphabetic() && self.chars[index] != '_' {
            return false;
        }
        let mut seen = index;
        while seen < self.chars.len() {
            let ch = self.chars[seen];
            if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-') {
                seen += 1;
            } else {
                break;
            }
        }
        while seen < self.chars.len() && self.chars[seen].is_whitespace() {
            seen += 1;
        }
        seen < self.chars.len() && self.chars[seen] == ':'
    }

    fn is_eof(&self) -> bool {
        self.skip_ws_comments_view().is_none()
    }

    fn skip_ws_comments(&mut self) {
        loop {
            while let Some(ch) = self.peek() {
                if ch.is_whitespace() {
                    self.pos += 1;
                } else {
                    break;
                }
            }
            if self.check('#') {
                self.skip_line_comment();
                continue;
            }
            if self.check('/') && matches!(self.peek_n(1), Some('/')) {
                self.skip_line_comment();
                continue;
            }
            if self.check('/') && matches!(self.peek_n(1), Some('*')) {
                self.skip_block_comment();
                continue;
            }
            break;
        }
    }

    fn skip_line_comment(&mut self) {
        if self.check('#') {
            self.pos += 1;
            while self.peek().is_some_and(|ch| ch != '\n') {
                self.pos += 1;
            }
            return;
        }
        self.pos += 2;
        while self.peek().is_some_and(|ch| ch != '\n') {
            self.pos += 1;
        }
    }

    fn skip_block_comment(&mut self) {
        self.pos += 2;
        while self.peek().is_some() {
            if self.check('*') && matches!(self.peek_n(1), Some('/')) {
                self.pos += 2;
                return;
            }
            self.pos += 1;
        }
        panic!("unterminated block comment");
    }

    fn rest(&self) -> String {
        self.chars[self.pos..].iter().collect()
    }

    fn peek_n_from(&self, index: usize) -> Option<char> {
        self.chars.get(index).copied()
    }

    fn peek_n(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn check(&self, expected: char) -> bool {
        self.peek() == Some(expected)
    }

    fn skip_ws_comments_view(&self) -> Option<char> {
        let mut index = self.pos;
        loop {
            while index < self.chars.len() && self.chars[index].is_whitespace() {
                index += 1;
            }
            if index < self.chars.len() && self.chars[index] == '#' {
                while index < self.chars.len() && self.chars[index] != '\n' {
                    index += 1;
                }
                continue;
            }
            if index + 1 < self.chars.len()
                && self.chars[index] == '/'
                && self.chars[index + 1] == '/'
            {
                index += 2;
                while index < self.chars.len() && self.chars[index] != '\n' {
                    index += 1;
                }
                continue;
            }
            if index + 1 < self.chars.len()
                && self.chars[index] == '/'
                && self.chars[index + 1] == '*'
            {
                index += 2;
                while index + 1 < self.chars.len()
                    && !(self.chars[index] == '*' && self.chars[index + 1] == '/')
                {
                    index += 1;
                }
                index += 2;
                continue;
            }
            break;
        }
        self.chars.get(index).copied()
    }

    fn expect_keyword(&mut self, keyword: &str) -> Result<(), ParseError> {
        self.skip_ws_comments();
        let start = self.pos;
        let Some(ch) = self.peek() else {
            return Err(ParseError {
                message: format!("expected '{keyword}'"),
            });
        };
        if !ch.is_ascii_alphabetic() && ch != '_' {
            return Err(ParseError {
                message: format!("expected '{keyword}', found '{:?}'", self.peek()),
            });
        }
        let mut end = start;
        while let Some(next) = self.peek_n_from(end) {
            if next.is_ascii_alphanumeric() || matches!(next, '_' | '-') {
                end += 1;
            } else {
                break;
            }
        }
        let text: String = self.chars[start..end].iter().collect();
        if text == keyword {
            self.pos = end;
            Ok(())
        } else {
            Err(ParseError {
                message: format!("expected '{keyword}', found '{text}'"),
            })
        }
    }

    fn check_keyword(&self, keyword: &str) -> bool {
        let mut index = self.pos;
        while index < self.chars.len() && self.chars[index].is_whitespace() {
            index += 1;
        }
        let start = index;
        if index >= self.chars.len()
            || (!self.chars[index].is_ascii_alphabetic() && self.chars[index] != '_')
        {
            return false;
        }
        index += 1;
        while index < self.chars.len()
            && (self.chars[index].is_ascii_alphanumeric() || matches!(self.chars[index], '_' | '-'))
        {
            index += 1;
        }
        let text: String = self.chars[start..index].iter().collect();
        text == keyword
    }

    fn expect(&mut self, expected: char) -> Result<(), ParseError> {
        self.skip_ws_comments();
        if self.check(expected) {
            self.pos += 1;
            Ok(())
        } else {
            Err(ParseError {
                message: format!("expected '{}', found '{:?}'", expected, self.peek()),
            })
        }
    }
}
