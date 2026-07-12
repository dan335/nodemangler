//! Shared utility functions: graph I/O, operation registry, value type helpers.

use std::collections::HashMap;
use std::path::PathBuf;

use mangler_core::{
    graph::Graph,
    operations::{operation_list, Operation, OperationListItem},
    value::ValueType,
};

// ── File save helpers ──────────────────────────────────────────────────────

/// Save an image to a file, inferring the format from the file extension.
///
/// Picks a color format compatible with the target format via
/// `ColorFormat::default_for_image_format` (same defaults the "to file" node
/// falls back to) and encodes through `mangler_core`'s shared save path. A
/// bare `FloatImage::to_dynamic()` + `DynamicImage::save()` would produce
/// Rgb32F/Rgba32F for 3/4-channel images, which most encoders (PNG, JPEG,
/// GIF, BMP, ...) reject outright — this is what makes `--save out.png` work
/// for ordinary color images instead of only for f32-native formats like EXR.
pub(crate) fn save_image_to_file(img: &mangler_core::float_image::FloatImage, path: &PathBuf) -> Result<(), String> {
    let image_format = image::ImageFormat::from_path(path)
        .map_err(|e| format!("failed to determine image format from '{}': {e}", path.display()))?;
    let color_format = mangler_core::value::ColorFormat::default_for_image_format(&image_format);
    mangler_core::operations::images::outputs::save_image(
        path,
        img,
        &color_format,
        image_format,
        85,
        image::codecs::png::CompressionType::Fast,
    )
}

/// Save a non-image value to a file as pretty-printed JSON.
pub(crate) fn save_value_to_file(value: &mangler_core::value::Value, path: &PathBuf) -> Result<(), String> {
    let json_str = serde_json::to_string_pretty(&crate::format::json_value(value))
        .map_err(|e| format!("failed to serialize value: {}", e))?;
    std::fs::write(path, json_str).map_err(|e| format!("failed to write file: {}", e))
}

// ── Node lookup helpers ───────────────────────────────────────────────────

/// Build a "node not found" error message.
///
/// If the given `id` matches a node's custom display name, the error suggests
/// using the actual node ID instead. Otherwise it lists available node IDs.
pub(crate) fn node_not_found_error(graph: &Graph, id: &str) -> String {
    // Check if the user passed a display name instead of a node ID.
    for (node_id, node) in &graph.nodes {
        if let Some(ref name) = node.custom_name {
            if name.eq_ignore_ascii_case(id) {
                return format!(
                    "node '{}' not found — '{}' is a display name, use node ID '{}' instead",
                    id, name, node_id
                );
            }
        }
    }

    // Generic message with available node IDs.
    if graph.nodes.is_empty() {
        format!("node '{}' not found — graph has no nodes", id)
    } else {
        let mut ids: Vec<&str> = graph.nodes.keys().map(|s| s.as_str()).collect();
        ids.sort();
        format!(
            "node '{}' not found — available node IDs: {}",
            id,
            ids.join(", ")
        )
    }
}

/// Reject a caller-supplied node `id` that already exists in `graph`.
///
/// `graph.add_node` overwrites any existing node sharing the same ID, which
/// silently drops it along with every connection that pointed at it. Callers
/// that accept an explicit `--id` (add-node, add-subgraph) must check this
/// before adding so a typo'd/reused ID surfaces as an error instead of a
/// silent, dangling-connection-producing overwrite.
pub(crate) fn reject_existing_id(graph: &Graph, id: Option<&str>) -> Result<(), String> {
    if let Some(id) = id {
        if graph.nodes.contains_key(id) {
            return Err(format!(
                "node '{}' already exists — choose a different --id or remove it first",
                id
            ));
        }
    }
    Ok(())
}

// ── Operation & type conversion helpers ───────────────────────────────────

/// Get the serde variant name for an Operation (e.g. "OpNumberMathAdd").
pub(crate) fn op_variant_name(op: &Operation) -> String {
    serde_json::to_string(op)
        .unwrap_or_default()
        .trim_matches('"')
        .to_string()
}

/// List type names a ValueType accepts conversion from (excluding self and Trigger).
pub(crate) fn accepted_conversions(vt: &ValueType) -> Vec<String> {
    vt.valid_conversions_from().iter()
        .filter(|t| **t != *vt && **t != ValueType::Trigger)
        .map(|t| value_type_name(t).to_string())
        .collect()
}

/// List type names a ValueType can convert to (excluding self and Trigger).
pub(crate) fn output_conversions(vt: &ValueType) -> Vec<String> {
    vt.valid_conversions().iter()
        .filter(|t| **t != *vt && **t != ValueType::Trigger)
        .map(|t| value_type_name(t).to_string())
        .collect()
}

// ── Graph load/save helpers ───────────────────────────────────────────────────

/// Load a graph from a JSON file with no UI channels.
///
/// Load anomalies (file saved by a newer NodeMangler, unknown nodes replaced
/// with placeholders — see `mangler_core::saved_nodes`) go to stderr: a
/// headless run has no banner to show, and silently computing defaults for
/// placeholder nodes would be worse than a warning.
pub(crate) fn load_graph(path: &PathBuf) -> Result<Graph, String> {
    let graph = Graph::load(path.clone(), None, None, false).map_err(|e| e.0)?;

    if let Some(report) = &graph.load_report {
        if report.is_newer_than_app {
            eprintln!(
                "warning: {} was saved with NodeMangler {} (this is {}); \
                 any save will restamp it with this version",
                path.display(),
                report.file_version,
                mangler_core::APP_VERSION,
            );
        }
        if !report.unknown_node_names.is_empty() {
            eprintln!(
                "warning: {} contains {} unknown node(s) preserved as placeholders \
                 (they will not run): {}",
                path.display(),
                report.unknown_node_names.len(),
                report.unknown_node_names.join(", "),
            );
        }
    }

    Ok(graph)
}

/// Serialize a graph and write it to a JSON file.
pub(crate) fn save_graph(graph: &Graph, path: &PathBuf) -> Result<(), String> {
    let save_data = graph.to_save_data();
    let json = serde_json::to_string_pretty(&save_data).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

// ── Operation registry helpers ────────────────────────────────────────────────

/// Recursively flatten the `operation_list()` tree into `(short_path, Operation)` pairs.
///
/// The path is built by joining category names with `/`, e.g. `numbers/arithmetic/add`.
/// Spaces in names are replaced with underscores so paths are CLI-friendly without quoting.
pub(crate) fn flatten_ops(items: &[OperationListItem], prefix: &str) -> Vec<(String, Operation)> {
    let mut result = Vec::new();
    for item in items {
        match item {
            OperationListItem::Category { name, operation_list_items } => {
                let slug = name.replace(' ', "_");
                let new_prefix = if prefix.is_empty() {
                    slug
                } else {
                    format!("{prefix}/{slug}")
                };
                result.extend(flatten_ops(operation_list_items, &new_prefix));
            }
            OperationListItem::Operation { operation } => {
                let op_name = operation.settings().name.replace(' ', "_");
                let path = if prefix.is_empty() {
                    op_name
                } else {
                    format!("{prefix}/{op_name}")
                };
                result.push((path, operation.clone()));
            }
            OperationListItem::Subgraph => {}
        }
    }
    result
}

/// Resolve an operation type string to an `Operation` variant.
///
/// Accepts either the short path (`numbers/arithmetic/add`, case-insensitive)
/// or the full serde variant name (`OpNumberMathAdd`).
pub(crate) fn resolve_op(type_str: &str) -> Result<Operation, String> {
    let all_ops = flatten_ops(&operation_list(), "");

    // Try short path first (case-insensitive, spaces normalized to underscores).
    let by_path: HashMap<String, Operation> =
        all_ops.iter().map(|(p, op)| (p.to_lowercase(), op.clone())).collect();
    let normalized = type_str.to_lowercase().replace(' ', "_");
    if let Some(op) = by_path.get(&normalized) {
        return Ok(op.clone());
    }

    // Try full variant name via serde round-trip.
    let json = format!("\"{}\"", type_str);
    if let Ok(op) = serde_json::from_str::<Operation>(&json) {
        return Ok(op);
    }

    Err(format!(
        "unknown operation '{}' — run `mangle show-ops` to see all types",
        type_str
    ))
}

/// Parse a `node-id:index` slot string into `(node_id, index)`.
pub(crate) fn parse_slot(s: &str) -> Result<(String, usize), String> {
    // Split on the last `:` so node IDs that contain `:` still work.
    let colon = s.rfind(':').ok_or_else(|| {
        format!("expected <node-id>:<index>, got '{s}'")
    })?;
    let node_id = s[..colon].to_string();
    let index: usize = s[colon + 1..]
        .parse()
        .map_err(|_| format!("invalid index in '{s}'"))?;
    Ok((node_id, index))
}

// ── Value type name helper ────────────────────────────────────────────────

/// Return the canonical lowercase CLI name for a ValueType.
pub(crate) fn value_type_name(vt: &ValueType) -> &'static str {
    match vt {
        ValueType::Bool => "bool",
        ValueType::Integer => "int",
        ValueType::Decimal => "decimal",
        ValueType::Text => "text",
        ValueType::Color => "color",
        ValueType::Path => "path",
        ValueType::Image => "image",
        ValueType::Trigger => "trigger",
        ValueType::BlendMode => "blendmode",
        ValueType::EdgeMode => "edgemode",
        ValueType::ColorSpace => "colorspace",
        ValueType::FilterType => "filtertype",
        ValueType::ImageType => "imagetype",
        ValueType::ColorFormat => "colorformat",
        ValueType::NoiseWorleyDistanceFunction => "worleydistance",
        ValueType::TextHAlign => "texthalign",
        ValueType::TextVAlign => "textvalign",
        ValueType::ExportPreset => "exportpreset",
        ValueType::Curve => "curve",
    }
}

// ── Enum type helpers ─────────────────────────────────────────────────────

/// All enum-like value types that users can set via the CLI.
/// Canonical lowercase enum type names shown in output.
pub(crate) const ENUM_TYPE_NAMES: &[&str] = &[
    "blendmode", "edgemode", "colorspace", "filtertype", "imagetype",
    "colorformat", "worleydistance", "texthalign", "textvalign", "exportpreset",
];

/// Legacy PascalCase aliases accepted as input prefixes (mapped to canonical names).
pub(crate) const ENUM_TYPE_ALIASES: &[(&str, &str)] = &[
    ("BlendMode", "blendmode"),
    ("EdgeMode", "edgemode"),
    ("ColorSpace", "colorspace"),
    ("FilterType", "filtertype"),
    ("ImageType", "imagetype"),
    ("ColorFormat", "colorformat"),
    ("NoiseWorleyDistanceFunction", "worleydistance"),
    ("TextHAlign", "texthalign"),
    ("TextVAlign", "textvalign"),
    ("ExportPreset", "exportpreset"),
];

/// Extract the serialized variant name from a `Value` that serializes as
/// `{"Tag": "variant string"}` — true of every enum-like value type, whether
/// via plain derive (e.g. `{"ColorSpace":"Oklab"}`) or a custom serializer
/// (e.g. `FilterType`'s `{"FilterType":"lanczos3"}`, `ImageType`'s
/// `{"ImageType":"avif"}`).
fn value_variant_name(value: &mangler_core::value::Value) -> String {
    let json = serde_json::to_value(value).expect("Value always serializes");
    json.as_object()
        .and_then(|obj| obj.values().next())
        .and_then(|v| v.as_str())
        .expect("enum-wrapped Value serializes as {Tag: string}")
        .to_string()
}

/// Return the valid variant names for an enum-like value type, or None if unknown.
///
/// Derived from each type's own `types()` (or, for `FilterType` — an external
/// `image` crate type with no `types()` of its own — `filter_type_variants()`)
/// rather than hand-copied, so a variant added in `mangler_core` shows up here
/// automatically instead of silently drifting out of sync (this happened
/// twice: colorspace was missing 5 of 14 variants, imagetype was missing "avif").
pub(crate) fn enum_variants(type_name: &str) -> Option<Vec<String>> {
    use mangler_core::color::blend::BlendMode;
    use mangler_core::color::color_spaces::ColorSpace;
    use mangler_core::operations::images::noise::cellular::worley_distance::NoiseWorleyDistanceFunction;
    use mangler_core::value::{
        filter_type_variants, ColorFormat, EdgeMode, ExportPreset, ImageType, TextHAlign, TextVAlign, Value,
    };

    match type_name.to_lowercase().as_str() {
        "blendmode" => Some(BlendMode::types().into_iter().map(|v| value_variant_name(&Value::BlendMode(v))).collect()),
        "colorspace" => Some(ColorSpace::types().into_iter().map(|v| value_variant_name(&Value::ColorSpace(v))).collect()),
        "filtertype" => Some(filter_type_variants().into_iter().map(|v| value_variant_name(&Value::FilterType(v))).collect()),
        "imagetype" => Some(ImageType::types().into_iter().map(|t| value_variant_name(&Value::ImageType(t.format()))).collect()),
        "colorformat" => Some(ColorFormat::types().into_iter().map(|v| value_variant_name(&Value::ColorFormat(v))).collect()),
        "worleydistance" | "noiseworleydistancefunction" => Some(
            NoiseWorleyDistanceFunction::types().into_iter().map(|v| value_variant_name(&Value::NoiseWorleyDistanceFunction(v))).collect(),
        ),
        "edgemode" => Some(EdgeMode::types().into_iter().map(|v| value_variant_name(&Value::EdgeMode(v))).collect()),
        "texthalign" => Some(TextHAlign::types().into_iter().map(|v| value_variant_name(&Value::TextHAlign(v))).collect()),
        "textvalign" => Some(TextVAlign::types().into_iter().map(|v| value_variant_name(&Value::TextVAlign(v))).collect()),
        "exportpreset" => Some(ExportPreset::types().into_iter().map(|v| value_variant_name(&Value::ExportPreset(v))).collect()),
        _ => None,
    }
}

/// Return the enum type name for a ValueType, if it's an enum type.
pub(crate) fn value_type_enum_name(vt: &ValueType) -> Option<&'static str> {
    match vt {
        ValueType::BlendMode => Some("blendmode"),
        ValueType::EdgeMode => Some("edgemode"),
        ValueType::ColorSpace => Some("colorspace"),
        ValueType::FilterType => Some("filtertype"),
        ValueType::ImageType => Some("imagetype"),
        ValueType::ColorFormat => Some("colorformat"),
        ValueType::NoiseWorleyDistanceFunction => Some("worleydistance"),
        ValueType::TextHAlign => Some("texthalign"),
        ValueType::TextVAlign => Some("textvalign"),
        ValueType::ExportPreset => Some("exportpreset"),
        _ => None,
    }
}

/// Resolve a type name to a canonical enum type name, accepting both canonical and legacy aliases.
pub(crate) fn resolve_enum_type_name(name: &str) -> Option<&'static str> {
    ENUM_TYPE_NAMES.iter().find(|t| t.eq_ignore_ascii_case(name)).copied()
        .or_else(|| ENUM_TYPE_ALIASES.iter().find(|(alias, _)| alias.eq_ignore_ascii_case(name)).map(|(_, canon)| *canon))
}

/// Collect top-level categories with counts from the flattened ops list.
pub(crate) fn collect_categories(all_ops: &[(String, Operation)]) -> Vec<(String, usize)> {
    let mut counts: HashMap<String, usize> = HashMap::new();
    for (path, _) in all_ops {
        let cat = path.split('/').next().unwrap_or(path).to_string();
        *counts.entry(cat).or_insert(0) += 1;
    }
    let mut cats: Vec<(String, usize)> = counts.into_iter().collect();
    cats.sort_by(|a, b| a.0.cmp(&b.0));
    cats
}

/// Score an operation against search terms for fuzzy ranked matching.
///
/// `haystack_parts` is `(path, variant, description)`, all lowercase.
/// `terms` are the lowercase search terms split on whitespace.
///
/// Scoring heuristic per term:
/// - Exact path segment match (term equals a `/`-delimited segment): +10 points
/// - Path contains term (substring): +5 points
/// - Variant exact match: +8 points
/// - Variant contains term: +4 points
/// - Description contains term: +2 points
///
/// If any term matches nothing, the total score is 0 (AND semantics).
pub(crate) fn score_op(haystack_parts: (&str, &str, &str), terms: &[String]) -> u32 {
    let (path, variant, description) = haystack_parts;
    let mut total: u32 = 0;

    for term in terms {
        let mut term_score: u32 = 0;

        // Exact path segment match (+10).
        for segment in path.split('/') {
            if segment == term {
                term_score += 10;
                break;
            }
        }

        // Path contains term (+5), only if no exact segment match.
        if term_score == 0 && path.contains(term.as_str()) {
            term_score += 5;
        }

        // Variant exact match (+8).
        if variant == term {
            term_score += 8;
        } else if variant.contains(term.as_str()) {
            // Variant contains term (+4).
            term_score += 4;
        }

        // Description contains term (+2).
        if description.contains(term.as_str()) {
            term_score += 2;
        }

        // If any term matches nothing, the whole op is excluded.
        if term_score == 0 {
            return 0;
        }

        total += term_score;
    }

    total
}

// ── Test helpers (shared across test modules) ─────────────────────────────

/// Create a temporary graph file path. The caller is responsible for cleanup
/// via `std::fs::remove_file`.
#[cfg(test)]
pub(crate) fn temp_graph_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "mangle_test_{}_{}.mangler.json",
        label,
        std::process::id()
    ))
}

/// Create a temp file with an empty graph and return its path.
#[cfg(test)]
pub(crate) fn create_temp_graph(label: &str) -> PathBuf {
    let path = temp_graph_path(label);
    let _ = std::fs::remove_file(&path);
    crate::commands::cmd_new(path.clone(), false).unwrap();
    path
}

#[cfg(test)]
#[path = "helpers_tests.rs"]
mod tests;
