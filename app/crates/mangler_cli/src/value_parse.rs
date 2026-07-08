//! Value parsing (`Type:value` format) and display.

use std::path::PathBuf;

use mangler_core::{color::Color, value::Value};

use crate::helpers::{enum_variants, resolve_enum_type_name};

// ── Typed value parser ────────────────────────────────────────────────────────

/// Parse a `Type:value` string into a `Value`.
///
/// Supports two formats:
///   1. **Typed prefix** — `Type:value` where `Type` is a known prefix (see table below).
///      The split happens on the *first* colon, so values like `path:C:\foo` work correctly.
///   2. **JSON fallback** — any valid serde JSON representation of `Value` (e.g. `{"Decimal":3.14}`).
///
/// Type prefixes (case-insensitive for simple types, case-insensitive for enum types):
///   `bool`, `int`, `decimal`, `text`, `color` (r,g,b,a), `path`,
///   `blendmode`, `colorspace`, `filtertype`, `imagetype`, `colorformat`,
///   `worleydistance`, `edgemode`, `texthalign`, `textvalign`, `exportpreset`.
pub(crate) fn parse_typed_value(s: &str) -> Result<Value, String> {
    // Try Type:value format — split on first colon.
    if let Some(colon_pos) = s.find(':') {
        let prefix = &s[..colon_pos];
        let rest = &s[colon_pos + 1..];

        // Simple types (case-insensitive prefix).
        match prefix.to_lowercase().as_str() {
            "bool" => {
                let b = rest.parse::<bool>().map_err(|_| {
                    format!("invalid bool value '{}' — expected true or false", rest)
                })?;
                return Ok(Value::Bool(b));
            }
            "int" => {
                let n = rest.parse::<i32>().map_err(|_| {
                    format!("invalid integer value '{}' — expected a 32-bit integer", rest)
                })?;
                return Ok(Value::Integer(n));
            }
            "decimal" => {
                let f = rest.parse::<f32>().map_err(|_| {
                    format!("invalid decimal value '{}' — expected a number", rest)
                })?;
                return Ok(Value::Decimal(f));
            }
            "text" => {
                return Ok(Value::Text(rest.to_string()));
            }
            "path" => {
                return Ok(Value::Path(PathBuf::from(rest)));
            }
            "color" => {
                let parts: Vec<&str> = rest.split(',').collect();
                if parts.len() != 4 {
                    return Err(format!(
                        "invalid color '{}' — expected 4 comma-separated floats (r,g,b,a), got {}",
                        rest,
                        parts.len()
                    ));
                }
                let vals: Vec<f32> = parts
                    .iter()
                    .enumerate()
                    .map(|(i, p)| {
                        p.trim().parse::<f32>().map_err(|_| {
                            format!("invalid color component [{}]: '{}' is not a number", i, p.trim())
                        })
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                return Ok(Value::Color(Color {
                    r: vals[0],
                    g: vals[1],
                    b: vals[2],
                    a: vals[3],
                }));
            }
            _ => {}
        }

        // Enum types — match case-insensitively against canonical names and legacy aliases.
        if let Some(canonical) = resolve_enum_type_name(prefix) {
            // Validate the variant exists.
            let variants = enum_variants(canonical).unwrap_or_default();
            let matched_variant = variants.iter().find(|v| v.eq_ignore_ascii_case(rest));
            // Map canonical lowercase name to the serde PascalCase name for JSON deser.
            let serde_name = match canonical {
                "blendmode" => "BlendMode",
                "edgemode" => "EdgeMode",
                "colorspace" => "ColorSpace",
                "filtertype" => "FilterType",
                "imagetype" => "ImageType",
                "colorformat" => "ColorFormat",
                "worleydistance" => "NoiseWorleyDistanceFunction",
                "texthalign" => "TextHAlign",
                "textvalign" => "TextVAlign",
                "exportpreset" => "ExportPreset",
                other => other,
            };
            match matched_variant {
                Some(variant) => {
                    // Deserialize via JSON: {"EnumType":"Variant"}
                    let json = format!("{{\"{serde_name}\":\"{variant}\"}}");
                    return serde_json::from_str::<Value>(&json).map_err(|e| {
                        format!("failed to parse {canonical}:{variant}: {e}")
                    });
                }
                None => {
                    return Err(format!(
                        "unknown {canonical} variant '{}' — valid values: {}",
                        rest,
                        variants.join(", ")
                    ));
                }
            }
        }
    }

    // JSON fallback.
    serde_json::from_str::<Value>(s).map_err(|e| {
        format!(
            "could not parse value '{}' — use Type:value format (e.g. decimal:3.14, bool:true, \
             color:1.0,0.0,0.0,1.0) or JSON (e.g. {{\"Decimal\":3.14}}). \
             Run `mangle show-values` for the full format reference. JSON error: {}",
            s, e
        )
    })
}

// ── Value display ─────────────────────────────────────────────────────────────

/// Return a concise human-readable representation of a `Value`.
pub(crate) fn display_value(value: &Value) -> String {
    match value {
        Value::Image { data, .. } => format!("<image {}x{}>", data.width(), data.height()),
        _ => serde_json::to_string(value).unwrap_or_else(|_| format!("{:?}", value)),
    }
}

#[cfg(test)]
#[path = "value_parse_tests.rs"]
mod tests;
