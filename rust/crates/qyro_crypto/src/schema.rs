//! A deliberately small JSON Schema validator for the committed vectors.
//!
//! Shared by every vector file in this crate, and shared on purpose: a second
//! copy is a second place for the rule below to be weakened without anyone
//! noticing.
//!
//! It understands `type` (`object`, `array`, `string`, `integer`),
//! `properties`, `required`, `additionalProperties: false`, `items`,
//! `minItems`, `const`, and `pattern` restricted to `^[0-9a-f]{N}$` — and it
//! **fails on any keyword it does not understand**.
//!
//! That last rule is the whole design. A validator that silently ignores an
//! unfamiliar keyword is worse than no validator: it reports success for
//! constraints it never checked, so a schema appears to be enforced while
//! quietly doing nothing. Adding `minLength` or `oneOf` to a schema must break
//! the build rather than pass unexamined.

use serde_json::Value;

/// Validates `value` against the subset of JSON Schema described above.
///
/// # Errors
///
/// Returns the first violation found, prefixed with the path to it.
pub(crate) fn validate(value: &Value, schema: &Value, path: &str) -> Result<(), String> {
    let object = schema.as_object().ok_or("schema node is not an object")?;

    for keyword in object.keys() {
        match keyword.as_str() {
            "type"
            | "properties"
            | "required"
            | "additionalProperties"
            | "items"
            | "minItems"
            | "const"
            | "pattern"
            | "description"
            | "$schema"
            | "title" => {}
            other => return Err(format!("{path}: unsupported schema keyword {other:?}")),
        }
    }

    match object.get("type").and_then(Value::as_str) {
        Some("object") => {
            let map = value
                .as_object()
                .ok_or_else(|| format!("{path}: expected an object"))?;

            let properties = object
                .get("properties")
                .and_then(Value::as_object)
                .ok_or_else(|| format!("{path}: object schema without properties"))?;

            if object.get("additionalProperties") != Some(&Value::Bool(false)) {
                return Err(format!("{path}: additionalProperties must be false"));
            }
            for key in map.keys() {
                if !properties.contains_key(key) {
                    return Err(format!("{path}: unknown property {key:?}"));
                }
            }

            let required = object
                .get("required")
                .and_then(Value::as_array)
                .ok_or_else(|| format!("{path}: object schema without required"))?;
            for name in required {
                let name = name.as_str().ok_or("required entries are strings")?;
                if !map.contains_key(name) {
                    return Err(format!("{path}: missing required property {name:?}"));
                }
            }
            // Every property is required: a partially specified vector is not a
            // vector anyone can interoperate against.
            for key in properties.keys() {
                if !required.iter().any(|name| name.as_str() == Some(key)) {
                    return Err(format!("{path}: property {key:?} is not required"));
                }
            }

            for (key, child) in properties {
                validate(&map[key], child, &format!("{path}/{key}"))?;
            }
        }
        Some("array") => {
            let items = value
                .as_array()
                .ok_or_else(|| format!("{path}: expected an array"))?;

            let schema_for_items = object
                .get("items")
                .ok_or_else(|| format!("{path}: array schema without items"))?;

            // A vector file with an empty list of cases would satisfy every
            // other rule here and prove nothing, so the count is part of the
            // schema rather than a convention.
            let minimum = object
                .get("minItems")
                .and_then(Value::as_u64)
                .ok_or_else(|| format!("{path}: array schema without minItems"))?;
            if (items.len() as u64) < minimum {
                return Err(format!(
                    "{path}: expected at least {minimum} items, got {}",
                    items.len()
                ));
            }

            for (index, item) in items.iter().enumerate() {
                validate(item, schema_for_items, &format!("{path}/{index}"))?;
            }
        }
        Some("string") => {
            let text = value
                .as_str()
                .ok_or_else(|| format!("{path}: expected a string"))?;
            if let Some(expected) = object.get("const") {
                if value != expected {
                    return Err(format!("{path}: expected const {expected}"));
                }
            }
            if let Some(pattern) = object.get("pattern").and_then(Value::as_str) {
                check_hex_pattern(text, pattern, path)?;
            }
        }
        Some("integer") => {
            let number = value
                .as_u64()
                .ok_or_else(|| format!("{path}: expected an integer"))?;
            if let Some(expected) = object.get("const").and_then(Value::as_u64) {
                if number != expected {
                    return Err(format!("{path}: expected const {expected}, got {number}"));
                }
            }
        }
        other => return Err(format!("{path}: unsupported type {other:?}")),
    }

    Ok(())
}

/// Enforces a `^[0-9a-f]{N}$` pattern without pulling in a regex engine.
///
/// The schema is checked to use only this shape, so there is no gap between
/// what the pattern says and what is enforced.
fn check_hex_pattern(text: &str, pattern: &str, path: &str) -> Result<(), String> {
    let body = pattern
        .strip_prefix("^[0-9a-f]{")
        .and_then(|rest| rest.strip_suffix("}$"))
        .ok_or_else(|| format!("{path}: unsupported pattern {pattern:?}"))?;

    // `{N}` fixes a length; `{N,}` is a floor, for the one field whose length is
    // the payload's rather than a constant.
    let (length, exact) = match body.strip_suffix(',') {
        Some(minimum) => (minimum, false),
        None => (body, true),
    };
    let length: usize = length
        .parse()
        .map_err(|_| format!("{path}: unsupported pattern length {body:?}"))?;

    if exact && text.len() != length {
        return Err(format!(
            "{path}: expected {length} hex characters, got {}",
            text.len()
        ));
    }
    if !exact && text.len() < length {
        return Err(format!(
            "{path}: expected at least {length} hex characters, got {}",
            text.len()
        ));
    }
    if !text.len().is_multiple_of(2) {
        return Err(format!("{path}: hex must be byte-aligned"));
    }
    if !text
        .bytes()
        .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(format!("{path}: not lowercase hex"));
    }
    Ok(())
}
