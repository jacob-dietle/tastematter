//! Boa JS executor: registers graph tools as JavaScript functions
//! and evaluates LLM-generated code in a sandboxed context.
//!
//! Port of `apps/codemode-graph/src/graph-tools.ts` into Boa engine.

use boa_engine::{
    js_string,
    native_function::NativeFunction,
    object::JsObject,
    property::{Attribute, PropertyKey},
    Context, JsArgs, JsError, JsNativeError, JsValue, Source,
};
use regex::Regex;
use std::collections::{HashSet, VecDeque};

use super::{extract_node_name, CorpusSnapshot};

/// Result of executing JavaScript code in the graph sandbox.
#[derive(Debug, Clone)]
pub struct ExecResult {
    pub result: Option<String>,
    pub error: Option<String>,
}

/// Build a Boa JS context with `codemode.*` functions registered,
/// then evaluate the given code and return the result.
pub fn execute_graph_code(snapshot: &CorpusSnapshot, code: &str) -> ExecResult {
    let mut ctx = Context::default();
    register_graph_tools(&mut ctx, snapshot);

    match ctx.eval(Source::from_bytes(code)) {
        Ok(val) => {
            let s = val
                .to_string(&mut ctx)
                .map(|js_str| js_str.to_std_string_escaped())
                .unwrap_or_else(|_| "undefined".to_string());
            ExecResult {
                result: Some(s),
                error: None,
            }
        }
        Err(e) => ExecResult {
            result: None,
            error: Some(format!("{e}")),
        },
    }
}

// =============================================================================
// HELPER: Convert Rust values to Boa JsValue
// =============================================================================

/// Convert a serde_json::Value to a JsValue in the Boa context.
fn json_to_js(val: &serde_json::Value, ctx: &mut Context) -> JsValue {
    match val {
        serde_json::Value::Null => JsValue::null(),
        serde_json::Value::Bool(b) => JsValue::from(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                JsValue::from(i as f64)
            } else {
                JsValue::from(n.as_f64().unwrap_or(0.0))
            }
        }
        serde_json::Value::String(s) => JsValue::from(js_string!(s.as_str())),
        serde_json::Value::Array(arr) => {
            let js_arr = boa_engine::object::builtins::JsArray::new(ctx);
            for item in arr {
                js_arr.push(json_to_js(item, ctx), ctx).unwrap();
            }
            js_arr.into()
        }
        serde_json::Value::Object(map) => {
            let obj = JsObject::default();
            for (k, v) in map {
                obj.set(js_string!(k.as_str()), json_to_js(v, ctx), true, ctx)
                    .unwrap();
            }
            JsValue::from(obj)
        }
    }
}

/// Convert a serde_yaml::Value to serde_json::Value (for frontmatter).
fn yaml_to_json(val: &serde_yaml::Value) -> serde_json::Value {
    match val {
        serde_yaml::Value::Null => serde_json::Value::Null,
        serde_yaml::Value::Bool(b) => serde_json::Value::Bool(*b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_json::Value::Number(i.into())
            } else if let Some(f) = n.as_f64() {
                serde_json::Number::from_f64(f)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null)
            } else {
                serde_json::Value::Null
            }
        }
        serde_yaml::Value::String(s) => serde_json::Value::String(s.clone()),
        serde_yaml::Value::Sequence(seq) => {
            serde_json::Value::Array(seq.iter().map(yaml_to_json).collect())
        }
        serde_yaml::Value::Mapping(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                let key = match k {
                    serde_yaml::Value::String(s) => s.clone(),
                    other => format!("{:?}", other),
                };
                obj.insert(key, yaml_to_json(v));
            }
            serde_json::Value::Object(obj)
        }
        serde_yaml::Value::Tagged(tagged) => yaml_to_json(&tagged.value),
    }
}

/// Get a string property from a JS object.
fn get_str(obj: &JsObject, key: &str, ctx: &mut Context) -> Option<String> {
    obj.get(js_string!(key), ctx).ok().and_then(|v| {
        if v.is_undefined() || v.is_null() {
            None
        } else {
            v.to_string(ctx).ok().map(|s| s.to_std_string_escaped())
        }
    })
}

/// Get a number property from a JS object, with a default.
fn get_num(obj: &JsObject, key: &str, default: f64, ctx: &mut Context) -> f64 {
    obj.get(js_string!(key), ctx)
        .ok()
        .and_then(|v| {
            if v.is_undefined() || v.is_null() {
                None
            } else {
                v.to_number(ctx).ok()
            }
        })
        .unwrap_or(default)
}

/// Find the file path for a given node name.
fn find_path_for_node(snapshot: &CorpusSnapshot, node_name: &str) -> Option<String> {
    snapshot
        .files
        .keys()
        .find(|p| extract_node_name(p) == node_name)
        .cloned()
}

// =============================================================================
// TOOL REGISTRATION
// =============================================================================

/// Register `codemode.graph_search`, `codemode.graph_traverse`,
/// `codemode.graph_read`, `codemode.graph_query` as JS functions.
///
/// # Safety
/// Uses raw pointer to snapshot. Safe because closures only execute during
/// `ctx.eval()` within `execute_graph_code`, where the snapshot reference is valid.
fn register_graph_tools(ctx: &mut Context, snapshot: &CorpusSnapshot) {
    let codemode = JsObject::default();

    // Raw pointer is Copy — safe because snapshot outlives the Boa context.
    // The context (and all closures) are dropped before execute_graph_code returns.
    let snap_ptr: *const CorpusSnapshot = snapshot;

    // --- graph_search ---
    {
        let search_fn = NativeFunction::from_copy_closure(move |_this, args, ctx| {
            // SAFETY: snap_ptr is valid for the lifetime of execute_graph_code
            let snap = unsafe { &*snap_ptr };
            let input = args.get_or_undefined(0);
            let obj = input.as_object().ok_or_else(|| {
                JsError::from_native(
                    JsNativeError::typ().with_message("graph_search expects an object"),
                )
            })?;

            let pattern = get_str(obj, "pattern", ctx).unwrap_or_default();
            let scope = get_str(obj, "scope", ctx).unwrap_or_else(|| "both".to_string());
            let max_results = get_num(obj, "maxResults", 20.0, ctx) as usize;

            let re = Regex::new(&pattern).map_err(|e| {
                JsError::from_native(
                    JsNativeError::error().with_message(format!("Invalid regex: {e}")),
                )
            })?;

            let mut results: Vec<serde_json::Value> = Vec::new();

            for (path, file) in &snap.files {
                let mut matches = Vec::new();
                let mut score: f64 = 0.0;

                if scope == "content" || scope == "both" {
                    for (i, line) in file.content.lines().enumerate() {
                        if re.is_match(line) {
                            matches.push(serde_json::json!({
                                "line": i + 1,
                                "content": line.trim(),
                            }));
                            score += 1.0;
                        }
                    }
                }

                if scope == "frontmatter" || scope == "both" {
                    let fm_str =
                        serde_json::to_string(&yaml_to_json(&file.frontmatter)).unwrap_or_default();
                    let fm_count = re.find_iter(&fm_str).count();
                    if fm_count > 0 {
                        score += fm_count as f64 * 2.0;
                        matches.push(serde_json::json!({
                            "line": 0,
                            "content": format!("[frontmatter] {} match(es)", fm_count),
                        }));
                    }
                }

                if score > 0.0 {
                    results.push(serde_json::json!({
                        "path": path,
                        "matches": matches,
                        "score": score,
                        "frontmatter": yaml_to_json(&file.frontmatter),
                    }));
                }
            }

            results.sort_by(|a, b| {
                b["score"]
                    .as_f64()
                    .unwrap_or(0.0)
                    .partial_cmp(&a["score"].as_f64().unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });
            results.truncate(max_results);

            let json_val = serde_json::Value::Array(results);
            Ok(json_to_js(&json_val, ctx))
        });

        codemode
            .set(
                js_string!("graph_search"),
                search_fn.to_js_function(ctx.realm()),
                true,
                ctx,
            )
            .unwrap();
    }

    // --- graph_traverse ---
    {
        let traverse_fn = NativeFunction::from_copy_closure(move |_this, args, ctx| {
            let snap = unsafe { &*snap_ptr };
            let input = args.get_or_undefined(0);
            let obj = input.as_object().ok_or_else(|| {
                JsError::from_native(
                    JsNativeError::typ().with_message("graph_traverse expects an object"),
                )
            })?;

            let start = get_str(obj, "start", ctx).unwrap_or_default();
            let direction =
                get_str(obj, "direction", ctx).unwrap_or_else(|| "outbound".to_string());
            let max_depth = get_num(obj, "maxDepth", 2.0, ctx) as usize;

            // Get optional filter object
            let filter_domain = obj
                .get(js_string!("filter"), ctx)
                .ok()
                .and_then(|v| v.as_object().cloned())
                .and_then(|f| get_str(&f, "domain", ctx));
            let filter_status = obj
                .get(js_string!("filter"), ctx)
                .ok()
                .and_then(|v| v.as_object().cloned())
                .and_then(|f| get_str(&f, "status", ctx));

            let mut nodes = Vec::new();
            let mut edges = Vec::new();
            let mut visited = HashSet::new();
            let mut queue: VecDeque<(String, usize)> = VecDeque::new();

            queue.push_back((start.clone(), 0));
            visited.insert(start.clone());

            while let Some((name, depth)) = queue.pop_front() {
                let path = find_path_for_node(snap, &name);
                let file = path.as_ref().and_then(|p| snap.files.get(p));

                // Apply filter to non-start nodes
                if depth > 0 {
                    if let Some(file) = file {
                        if let Some(ref dom) = filter_domain {
                            let fm_domain = file
                                .frontmatter
                                .get("domain")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            if fm_domain != dom {
                                continue;
                            }
                        }
                        if let Some(ref stat) = filter_status {
                            let fm_status = file
                                .frontmatter
                                .get("status")
                                .and_then(|v| v.as_str())
                                .unwrap_or("");
                            if fm_status != stat {
                                continue;
                            }
                        }
                    }
                }

                // Skip phantom nodes
                let (path, file) = match (path, file) {
                    (Some(p), Some(f)) => (p, f),
                    _ => continue,
                };

                nodes.push(serde_json::json!({
                    "path": path,
                    "name": name,
                    "depth": depth,
                    "frontmatter": yaml_to_json(&file.frontmatter),
                }));

                if depth >= max_depth {
                    continue;
                }

                if let Some(link_entry) = snap.index.links.get(&name) {
                    let mut neighbors = Vec::new();
                    if direction == "outbound" || direction == "both" {
                        neighbors.extend(link_entry.outbound.iter().cloned());
                    }
                    if direction == "inbound" || direction == "both" {
                        neighbors.extend(link_entry.inbound.iter().cloned());
                    }

                    for neighbor in neighbors {
                        edges.push(serde_json::json!({ "from": name, "to": neighbor }));
                        if visited.insert(neighbor.clone()) {
                            queue.push_back((neighbor, depth + 1));
                        }
                    }
                }
            }

            let result = serde_json::json!({ "nodes": nodes, "edges": edges });
            Ok(json_to_js(&result, ctx))
        });

        codemode
            .set(
                js_string!("graph_traverse"),
                traverse_fn.to_js_function(ctx.realm()),
                true,
                ctx,
            )
            .unwrap();
    }

    // --- graph_read ---
    {
        let read_fn = NativeFunction::from_copy_closure(move |_this, args, ctx| {
            let snap = unsafe { &*snap_ptr };
            let input = args.get_or_undefined(0);
            let obj = input.as_object().ok_or_else(|| {
                JsError::from_native(
                    JsNativeError::typ().with_message("graph_read expects an object"),
                )
            })?;

            let path = get_str(obj, "path", ctx).unwrap_or_default();
            let section = get_str(obj, "section", ctx);
            let max_lines = get_num(obj, "maxLines", 0.0, ctx) as usize;

            let file = snap.files.get(&path).ok_or_else(|| {
                JsError::from_native(
                    JsNativeError::error().with_message(format!("File not found: {path}")),
                )
            })?;

            let node_name = extract_node_name(&path);
            let link_entry = snap.index.links.get(&node_name);

            let mut content = file.content.clone();

            // Extract section if requested
            if let Some(ref section_name) = section {
                let heading_re = Regex::new(r"^(#{1,6})\s+(.+)").unwrap();
                let mut in_section = false;
                let mut section_level = 0usize;
                let mut section_lines = Vec::new();

                for line in content.lines() {
                    if let Some(caps) = heading_re.captures(line) {
                        let level = caps[1].len();
                        let title = caps[2].trim();
                        if title == section_name.as_str() {
                            in_section = true;
                            section_level = level;
                            section_lines.push(line);
                            continue;
                        }
                        if in_section && level <= section_level {
                            break;
                        }
                    }
                    if in_section {
                        section_lines.push(line);
                    }
                }
                content = section_lines.join("\n");
            }

            // Truncate by max lines
            if max_lines > 0 {
                content = content
                    .lines()
                    .take(max_lines)
                    .collect::<Vec<_>>()
                    .join("\n");
            }

            let outbound: Vec<String> = link_entry.map(|e| e.outbound.clone()).unwrap_or_default();
            let inbound: Vec<String> = link_entry.map(|e| e.inbound.clone()).unwrap_or_default();

            let result = serde_json::json!({
                "path": path,
                "content": content,
                "frontmatter": yaml_to_json(&file.frontmatter),
                "outbound_links": outbound,
                "inbound_links": inbound,
            });
            Ok(json_to_js(&result, ctx))
        });

        codemode
            .set(
                js_string!("graph_read"),
                read_fn.to_js_function(ctx.realm()),
                true,
                ctx,
            )
            .unwrap();
    }

    // --- graph_query ---
    {
        let query_fn = NativeFunction::from_copy_closure(move |_this, args, ctx| {
            let snap = unsafe { &*snap_ptr };
            let input = args.get_or_undefined(0);
            let obj = input.as_object().ok_or_else(|| {
                JsError::from_native(
                    JsNativeError::typ().with_message("graph_query expects an object"),
                )
            })?;

            let limit = get_num(obj, "limit", 50.0, ctx) as usize;
            let sort_by = get_str(obj, "sort", ctx);

            // Extract filter object
            let filter_obj = obj
                .get(js_string!("filter"), ctx)
                .ok()
                .and_then(|v| v.as_object().cloned());

            let filter_status = filter_obj.as_ref().and_then(|f| get_str(f, "status", ctx));
            let filter_domain = filter_obj.as_ref().and_then(|f| get_str(f, "domain", ctx));
            let filter_name = filter_obj.as_ref().and_then(|f| get_str(f, "name", ctx));

            // Extract tags filter (can be array)
            let filter_tags: Option<Vec<String>> = filter_obj.as_ref().and_then(|f| {
                let val = f.get(js_string!("tags"), ctx).ok()?;
                if val.is_undefined() || val.is_null() {
                    return None;
                }
                let arr_obj = val.as_object()?;
                let len = arr_obj
                    .get(js_string!("length"), ctx)
                    .ok()?
                    .to_number(ctx)
                    .ok()? as usize;
                let mut tags = Vec::new();
                for i in 0..len {
                    if let Ok(item) = arr_obj.get(PropertyKey::from(i as u32), ctx) {
                        if let Ok(s) = item.to_string(ctx) {
                            tags.push(s.to_std_string_escaped());
                        }
                    }
                }
                Some(tags)
            });

            let mut matching_nodes = Vec::new();
            let name_re = filter_name.as_ref().map(|p| {
                Regex::new(&format!("(?i){}", p)).unwrap_or_else(|_| Regex::new("$^").unwrap())
            });

            for (path, file) in &snap.files {
                let fm = &file.frontmatter;

                // Apply filters
                if let Some(ref status) = filter_status {
                    let fm_status = fm.get("status").and_then(|v| v.as_str()).unwrap_or("");
                    if fm_status != status {
                        continue;
                    }
                }
                if let Some(ref domain) = filter_domain {
                    let fm_domain = fm.get("domain").and_then(|v| v.as_str()).unwrap_or("");
                    if fm_domain != domain {
                        continue;
                    }
                }
                if let Some(ref re) = name_re {
                    let fm_name = fm.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    if !re.is_match(fm_name) {
                        continue;
                    }
                }
                if let Some(ref required_tags) = filter_tags {
                    let file_tags: Vec<String> = fm
                        .get("tags")
                        .and_then(|v| v.as_sequence())
                        .map(|seq| {
                            seq.iter()
                                .filter_map(|t| t.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();
                    if !required_tags.iter().all(|t| file_tags.contains(t)) {
                        continue;
                    }
                }

                let node_name = extract_node_name(path);
                let link_entry = snap.index.links.get(&node_name);
                let fm_name = fm
                    .get("name")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_else(|| node_name.clone());

                matching_nodes.push(serde_json::json!({
                    "path": path,
                    "name": fm_name,
                    "status": fm.get("status").and_then(|v| v.as_str()).unwrap_or("unknown"),
                    "domain": fm.get("domain").and_then(|v| v.as_str()).unwrap_or("unknown"),
                    "tags": fm.get("tags").and_then(|v| v.as_sequence())
                        .map(|seq| seq.iter().filter_map(|t| t.as_str()).collect::<Vec<_>>())
                        .unwrap_or_default(),
                    "last_updated": fm.get("last_updated").and_then(|v| v.as_str()).unwrap_or(""),
                    "link_count": {
                        "outbound": link_entry.map(|e| e.outbound.len()).unwrap_or(0),
                        "inbound": link_entry.map(|e| e.inbound.len()).unwrap_or(0),
                    },
                }));
            }

            // Sort
            if let Some(ref sort) = sort_by {
                match sort.as_str() {
                    "name" => matching_nodes.sort_by(|a, b| {
                        a["name"]
                            .as_str()
                            .unwrap_or("")
                            .cmp(b["name"].as_str().unwrap_or(""))
                    }),
                    "status" => matching_nodes.sort_by(|a, b| {
                        a["status"]
                            .as_str()
                            .unwrap_or("")
                            .cmp(b["status"].as_str().unwrap_or(""))
                    }),
                    "last_updated" => matching_nodes.sort_by(|a, b| {
                        b["last_updated"]
                            .as_str()
                            .unwrap_or("")
                            .cmp(a["last_updated"].as_str().unwrap_or(""))
                    }),
                    _ => {}
                }
            }

            let total = matching_nodes.len();
            matching_nodes.truncate(limit);

            let result = serde_json::json!({
                "nodes": matching_nodes,
                "total": total,
            });
            Ok(json_to_js(&result, ctx))
        });

        codemode
            .set(
                js_string!("graph_query"),
                query_fn.to_js_function(ctx.realm()),
                true,
                ctx,
            )
            .unwrap();
    }

    // Register the codemode object globally
    ctx.register_global_property(
        js_string!("codemode"),
        codemode,
        Attribute::READONLY | Attribute::NON_ENUMERABLE | Attribute::PERMANENT,
    )
    .unwrap();
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::load_graph;
    use std::path::PathBuf;

    fn fixture_snapshot() -> CorpusSnapshot {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("tests/fixtures/test-graph");
        load_graph(p.to_str().unwrap())
    }

    // --- graph_search ---

    #[test]
    fn test_graph_search_basic() {
        let snap = fixture_snapshot();
        let result = execute_graph_code(
            &snap,
            r##"
            (() => {
                const results = codemode.graph_search({ pattern: "context" });
                return JSON.stringify(results.length);
            })()
        "##,
        );
        assert!(result.error.is_none(), "Error: {:?}", result.error);
        let count: usize = serde_json::from_str(result.result.as_ref().unwrap()).unwrap();
        assert!(count > 0, "Should find files matching 'context'");
    }

    #[test]
    fn test_graph_search_frontmatter_scope() {
        let snap = fixture_snapshot();
        let result = execute_graph_code(
            &snap,
            r##"
            (() => {
                const results = codemode.graph_search({ pattern: "canonical", scope: "frontmatter" });
                return JSON.stringify(results.map(r => r.path));
            })()
        "##,
        );
        assert!(result.error.is_none(), "Error: {:?}", result.error);
        let paths: Vec<String> = serde_json::from_str(result.result.as_ref().unwrap()).unwrap();
        assert!(
            paths.len() >= 3,
            "Should find canonical files in frontmatter"
        );
    }

    #[test]
    fn test_graph_search_max_results() {
        let snap = fixture_snapshot();
        let result = execute_graph_code(
            &snap,
            r##"
            (() => {
                const results = codemode.graph_search({ pattern: ".", maxResults: 2 });
                return JSON.stringify(results.length);
            })()
        "##,
        );
        assert!(result.error.is_none(), "Error: {:?}", result.error);
        let count: usize = serde_json::from_str(result.result.as_ref().unwrap()).unwrap();
        assert!(count <= 2, "Should respect maxResults limit");
    }

    // --- graph_traverse ---

    #[test]
    fn test_graph_traverse_outbound() {
        let snap = fixture_snapshot();
        let result = execute_graph_code(
            &snap,
            r##"
            (() => {
                const r = codemode.graph_traverse({ start: "context-engineering", direction: "outbound", maxDepth: 1 });
                return JSON.stringify(r.nodes.map(n => n.name));
            })()
        "##,
        );
        assert!(result.error.is_none(), "Error: {:?}", result.error);
        let names: Vec<String> = serde_json::from_str(result.result.as_ref().unwrap()).unwrap();
        assert!(names.contains(&"context-engineering".to_string()));
        assert!(names.contains(&"taste-operationalization".to_string()));
    }

    #[test]
    fn test_graph_traverse_inbound() {
        let snap = fixture_snapshot();
        let result = execute_graph_code(
            &snap,
            r##"
            (() => {
                const r = codemode.graph_traverse({ start: "context-engineering", direction: "inbound", maxDepth: 1 });
                return JSON.stringify(r.nodes.map(n => n.name));
            })()
        "##,
        );
        assert!(result.error.is_none(), "Error: {:?}", result.error);
        let names: Vec<String> = serde_json::from_str(result.result.as_ref().unwrap()).unwrap();
        // agentic-systems, taste-op, business-model all link TO context-engineering
        assert!(names.len() >= 2, "Should find inbound nodes");
    }

    #[test]
    fn test_graph_traverse_depth_limit() {
        let snap = fixture_snapshot();
        let result = execute_graph_code(
            &snap,
            r##"
            (() => {
                const r = codemode.graph_traverse({ start: "context-engineering", direction: "both", maxDepth: 0 });
                return JSON.stringify(r.nodes.length);
            })()
        "##,
        );
        assert!(result.error.is_none(), "Error: {:?}", result.error);
        let count: usize = serde_json::from_str(result.result.as_ref().unwrap()).unwrap();
        assert_eq!(count, 1, "Depth 0 should return only the start node");
    }

    // --- graph_read ---

    #[test]
    fn test_graph_read_basic() {
        let snap = fixture_snapshot();
        let result = execute_graph_code(
            &snap,
            r##"
            (() => {
                const r = codemode.graph_read({ path: "technical/context-engineering.md" });
                return JSON.stringify({
                    has_content: r.content.length > 0,
                    has_outbound: r.outbound_links.length > 0,
                    has_inbound: r.inbound_links.length > 0,
                    domain: r.frontmatter.domain,
                });
            })()
        "##,
        );
        assert!(result.error.is_none(), "Error: {:?}", result.error);
        let val: serde_json::Value = serde_json::from_str(result.result.as_ref().unwrap()).unwrap();
        assert_eq!(val["has_content"], true);
        assert_eq!(val["has_outbound"], true);
        assert_eq!(val["has_inbound"], true);
        assert_eq!(val["domain"], "technical");
    }

    #[test]
    fn test_graph_read_section() {
        let snap = fixture_snapshot();
        let result = execute_graph_code(
            &snap,
            r##"
            (() => {
                const r = codemode.graph_read({ path: "technical/context-engineering.md", section: "Core Principle" });
                return JSON.stringify(r.content);
            })()
        "##,
        );
        assert!(result.error.is_none(), "Error: {:?}", result.error);
        let content: String = serde_json::from_str(result.result.as_ref().unwrap()).unwrap();
        assert!(
            content.contains("context"),
            "Section should contain context-related text"
        );
    }

    #[test]
    fn test_graph_read_not_found() {
        let snap = fixture_snapshot();
        let result = execute_graph_code(
            &snap,
            r##"
            (() => {
                try {
                    codemode.graph_read({ path: "nonexistent/file.md" });
                    return "should_have_thrown";
                } catch (e) {
                    return JSON.stringify({ error: true });
                }
            })()
        "##,
        );
        assert!(result.error.is_none(), "Executor itself should not crash");
        let val: serde_json::Value = serde_json::from_str(result.result.as_ref().unwrap()).unwrap();
        assert_eq!(val["error"], true);
    }

    // --- graph_query ---

    #[test]
    fn test_graph_query_by_domain() {
        let snap = fixture_snapshot();
        let result = execute_graph_code(
            &snap,
            r##"
            (() => {
                const r = codemode.graph_query({ filter: { domain: "technical" } });
                return JSON.stringify({ total: r.total, names: r.nodes.map(n => n.name) });
            })()
        "##,
        );
        assert!(result.error.is_none(), "Error: {:?}", result.error);
        let val: serde_json::Value = serde_json::from_str(result.result.as_ref().unwrap()).unwrap();
        assert_eq!(val["total"], 2);
    }

    #[test]
    fn test_graph_query_by_status() {
        let snap = fixture_snapshot();
        let result = execute_graph_code(
            &snap,
            r##"
            (() => {
                const r = codemode.graph_query({ filter: { status: "canonical" } });
                return JSON.stringify(r.total);
            })()
        "##,
        );
        assert!(result.error.is_none(), "Error: {:?}", result.error);
        let count: usize = serde_json::from_str(result.result.as_ref().unwrap()).unwrap();
        assert_eq!(count, 3, "3 canonical files in fixture");
    }

    #[test]
    fn test_graph_query_by_tags() {
        let snap = fixture_snapshot();
        let result = execute_graph_code(
            &snap,
            r##"
            (() => {
                const r = codemode.graph_query({ filter: { tags: ["methodology", "taste"] } });
                return JSON.stringify(r.nodes.map(n => n.name));
            })()
        "##,
        );
        assert!(result.error.is_none(), "Error: {:?}", result.error);
        let names: Vec<String> = serde_json::from_str(result.result.as_ref().unwrap()).unwrap();
        // Only taste-operationalization has both tags (name from frontmatter is UPPER_CASE)
        assert!(
            names.iter().any(|n| n.to_lowercase().contains("taste")),
            "Names: {:?}",
            names
        );
    }

    #[test]
    fn test_graph_query_limit() {
        let snap = fixture_snapshot();
        let result = execute_graph_code(
            &snap,
            r##"
            (() => {
                const r = codemode.graph_query({ filter: {}, limit: 2 });
                return JSON.stringify(r.nodes.length);
            })()
        "##,
        );
        assert!(result.error.is_none(), "Error: {:?}", result.error);
        let count: usize = serde_json::from_str(result.result.as_ref().unwrap()).unwrap();
        assert!(count <= 2);
    }

    // --- composition + error handling ---

    #[test]
    fn test_composition_search_then_read() {
        let snap = fixture_snapshot();
        let result = execute_graph_code(
            &snap,
            r##"
            (() => {
                const results = codemode.graph_search({ pattern: "context" });
                const top = results[0];
                const detail = codemode.graph_read({ path: top.path });
                return JSON.stringify({
                    path: detail.path,
                    outbound_count: detail.outbound_links.length,
                    inbound_count: detail.inbound_links.length,
                });
            })()
        "##,
        );
        assert!(result.error.is_none(), "Error: {:?}", result.error);
        let val: serde_json::Value = serde_json::from_str(result.result.as_ref().unwrap()).unwrap();
        assert!(val["outbound_count"].as_u64().unwrap() > 0);
    }

    #[test]
    fn test_error_handling_bad_syntax() {
        let snap = fixture_snapshot();
        let result = execute_graph_code(&snap, "() => { return @@invalid; }");
        assert!(result.error.is_some(), "Should capture syntax error");
        assert!(result.result.is_none());
    }

    #[test]
    fn test_real_llm_pattern() {
        let snap = fixture_snapshot();
        let result = execute_graph_code(
            &snap,
            r##"
            (() => {
                const results = codemode.graph_search({ pattern: "context" });
                const top = results[0];
                const detail = codemode.graph_read({ path: top.path });
                return JSON.stringify({
                    path: detail.path,
                    status: detail.frontmatter.status,
                    outbound_count: detail.outbound_links.length,
                    inbound_count: detail.inbound_links.length,
                });
            })()
        "##,
        );
        assert!(result.error.is_none(), "Error: {:?}", result.error);
        let val: serde_json::Value = serde_json::from_str(result.result.as_ref().unwrap()).unwrap();
        // Top result has a path, status, and link counts — proves search→read composition works
        assert!(val["path"].as_str().unwrap().ends_with(".md"));
        assert!(
            val["outbound_count"].as_u64().unwrap() > 0
                || val["inbound_count"].as_u64().unwrap() > 0
        );
    }
}
