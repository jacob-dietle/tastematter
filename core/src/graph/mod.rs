//! Graph loader: walks markdown directories, parses YAML frontmatter,
//! extracts [[wiki-links]], and builds in-memory indexes.
//!
//! Port of `apps/codemode-graph/src/graph-loader.ts`.

pub mod executor;

use regex::Regex;
use serde_yaml::Value as YamlValue;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use walkdir::WalkDir;

// =============================================================================
// DATA STRUCTURES (mirrors codemode-graph/src/types.ts)
// =============================================================================

#[derive(Debug, Clone)]
pub struct CorpusFile {
    pub path: String,
    pub content: String,
    pub frontmatter: YamlValue,
}

#[derive(Debug, Clone, Default)]
pub struct LinkEntry {
    pub outbound: Vec<String>,
    pub inbound: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct GraphIndex {
    pub links: HashMap<String, LinkEntry>,
    pub tree: HashMap<String, Vec<String>>,
    pub tags: HashMap<String, Vec<String>>,
    pub domains: HashMap<String, Vec<String>>,
    pub statuses: HashMap<String, Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct CorpusSnapshot {
    pub root: String,
    pub files: HashMap<String, CorpusFile>,
    pub file_count: usize,
    pub loaded_at: String,
    pub index: GraphIndex,
}

// =============================================================================
// LOADING
// =============================================================================

const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".claude",
    "dist",
    ".next",
    ".svelte-kit",
];

/// Load a knowledge graph from a directory of markdown files.
///
/// Walks the directory tree, parses YAML frontmatter, extracts [[wiki-links]],
/// and builds indexes for links, tree structure, tags, domains, and statuses.
pub fn load_graph(root_path: &str) -> CorpusSnapshot {
    let root = Path::new(root_path);
    let mut files = HashMap::new();

    for entry in WalkDir::new(root)
        .into_iter()
        .filter_entry(|e| {
            // Skip directories in SKIP_DIRS
            if e.file_type().is_dir() {
                let name = e.file_name().to_string_lossy();
                return !SKIP_DIRS.contains(&name.as_ref());
            }
            true
        })
        .filter_map(|e| e.ok())
    {
        if !entry.file_type().is_file() {
            continue;
        }
        let abs_path = entry.path();
        if abs_path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }

        let rel_path = abs_path
            .strip_prefix(root)
            .unwrap_or(abs_path)
            .to_string_lossy()
            .replace('\\', "/");

        let raw = match fs::read_to_string(abs_path) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let (frontmatter, content) = parse_frontmatter(&raw);
        files.insert(
            rel_path.clone(),
            CorpusFile {
                path: rel_path,
                content,
                frontmatter,
            },
        );
    }

    let file_count = files.len();
    let index = build_index(&files);

    CorpusSnapshot {
        root: root_path.to_string(),
        files,
        file_count,
        loaded_at: chrono::Utc::now().to_rfc3339(),
        index,
    }
}

/// Extract the node name (filename stem) from a relative path.
///
/// `"technical/context-engineering.md"` → `"context-engineering"`
pub fn extract_node_name(path: &str) -> String {
    let base = path.rsplit('/').next().unwrap_or(path);
    base.strip_suffix(".md").unwrap_or(base).to_string()
}

/// Extract unique [[wiki-links]] from text.
///
/// `"See [[foo]] and [[bar]], also [[foo]]"` → `["foo", "bar"]`
pub fn extract_wiki_links(text: &str) -> Vec<String> {
    let re = Regex::new(r"\[\[([^\]]+)\]\]").unwrap();
    let mut seen = HashSet::new();
    let mut links = Vec::new();
    for cap in re.captures_iter(text) {
        let name = cap[1].to_string();
        if seen.insert(name.clone()) {
            links.push(name);
        }
    }
    links
}

/// Split markdown content into (frontmatter YAML, body content).
/// Returns (YamlValue, body_string). On parse failure, returns (Null, full_content).
fn parse_frontmatter(raw: &str) -> (YamlValue, String) {
    // Frontmatter must start with "---\n" at the very beginning
    if !raw.starts_with("---") {
        return (YamlValue::Null, raw.to_string());
    }

    // Find the closing "---"
    let after_first = &raw[3..];
    let closing = after_first.find("\n---");
    match closing {
        None => (YamlValue::Null, raw.to_string()),
        Some(pos) => {
            let yaml_str = &after_first[..pos];
            let body_start = 3 + pos + 4; // skip "---" + "\n---"
            let body = if body_start < raw.len() {
                // Skip leading newlines after closing ---
                raw[body_start..].trim_start_matches('\n').to_string()
            } else {
                String::new()
            };

            match serde_yaml::from_str::<YamlValue>(yaml_str) {
                Ok(val) => (val, body),
                Err(_) => (YamlValue::Null, raw.to_string()),
            }
        }
    }
}

/// Build all indexes from loaded files.
fn build_index(files: &HashMap<String, CorpusFile>) -> GraphIndex {
    let mut links: HashMap<String, LinkEntry> = HashMap::new();
    let mut tree: HashMap<String, Vec<String>> = HashMap::new();
    let mut tags: HashMap<String, Vec<String>> = HashMap::new();
    let mut domains: HashMap<String, Vec<String>> = HashMap::new();
    let mut statuses: HashMap<String, Vec<String>> = HashMap::new();

    for (path, file) in files {
        let node_name = extract_node_name(path);

        // Ensure link entry exists for this node
        if !links.contains_key(&node_name) {
            links.insert(node_name.clone(), LinkEntry::default());
        }

        // Extract wiki-links from frontmatter (serialized) + content
        let fm_str = serde_json::to_string(&file.frontmatter).unwrap_or_default();
        let all_text = format!("{}\n{}", fm_str, file.content);
        let outbound = extract_wiki_links(&all_text);

        links.get_mut(&node_name).unwrap().outbound = outbound.clone();

        // Register inbound links on target nodes
        for target in &outbound {
            links
                .entry(target.clone())
                .or_default()
                .inbound
                .push(node_name.clone());
        }

        // Tree index: directory → [file paths]
        let dir = if let Some(pos) = path.rfind('/') {
            &path[..pos]
        } else {
            "."
        };
        tree.entry(dir.to_string())
            .or_default()
            .push(path.clone());

        // Tag index from frontmatter
        if let Some(tag_seq) = file.frontmatter.get("tags").and_then(|v| v.as_sequence()) {
            for tag_val in tag_seq {
                if let Some(tag) = tag_val.as_str() {
                    tags.entry(tag.to_string())
                        .or_default()
                        .push(path.clone());
                }
            }
        }

        // Domain index
        if let Some(domain) = file.frontmatter.get("domain").and_then(|v| v.as_str()) {
            domains
                .entry(domain.to_string())
                .or_default()
                .push(path.clone());
        }

        // Status index
        if let Some(status) = file.frontmatter.get("status").and_then(|v| v.as_str()) {
            statuses
                .entry(status.to_string())
                .or_default()
                .push(path.clone());
        }
    }

    // Deduplicate inbound links (a node can be linked multiple times from same source)
    for entry in links.values_mut() {
        entry.inbound.sort();
        entry.inbound.dedup();
    }

    GraphIndex {
        links,
        tree,
        tags,
        domains,
        statuses,
    }
}

// =============================================================================
// TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn fixture_path() -> String {
        let mut p = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        p.push("tests/fixtures/test-graph");
        p.to_string_lossy().to_string()
    }

    // --- extract_node_name ---

    #[test]
    fn test_node_name_from_nested_path() {
        assert_eq!(
            extract_node_name("technical/context-engineering.md"),
            "context-engineering"
        );
    }

    #[test]
    fn test_node_name_from_flat_path() {
        assert_eq!(extract_node_name("README.md"), "README");
    }

    #[test]
    fn test_node_name_no_extension() {
        // Edge case: path without .md
        assert_eq!(extract_node_name("technical/notes"), "notes");
    }

    // --- extract_wiki_links ---

    #[test]
    fn test_extract_wiki_links_basic() {
        let links = extract_wiki_links("See [[foo]] and [[bar]]");
        assert_eq!(links, vec!["foo", "bar"]);
    }

    #[test]
    fn test_extract_wiki_links_dedup() {
        let links = extract_wiki_links("[[foo]] then [[foo]] again");
        assert_eq!(links, vec!["foo"]);
    }

    #[test]
    fn test_extract_wiki_links_empty() {
        let links = extract_wiki_links("No links here");
        assert!(links.is_empty());
    }

    #[test]
    fn test_extract_wiki_links_in_yaml() {
        let links = extract_wiki_links(r#"related: ["[[context-engineering]]", "[[business-model]]"]"#);
        assert_eq!(links, vec!["context-engineering", "business-model"]);
    }

    // --- parse_frontmatter ---

    #[test]
    fn test_parse_frontmatter_valid() {
        let raw = "---\ntitle: Test\ndomain: technical\n---\n\n# Body\nContent here";
        let (fm, body) = parse_frontmatter(raw);
        assert_eq!(fm["title"].as_str(), Some("Test"));
        assert_eq!(fm["domain"].as_str(), Some("technical"));
        assert!(body.contains("# Body"));
    }

    #[test]
    fn test_parse_frontmatter_missing() {
        let raw = "# No Frontmatter\nJust content";
        let (fm, body) = parse_frontmatter(raw);
        assert!(fm.is_null());
        assert!(body.contains("# No Frontmatter"));
    }

    #[test]
    fn test_parse_frontmatter_malformed() {
        let raw = "---\ninvalid: [yaml: broken\n---\n\n# Body";
        let (fm, body) = parse_frontmatter(raw);
        // Malformed → treat as no frontmatter, full content preserved
        assert!(fm.is_null());
    }

    // --- load_graph (integration) ---

    #[test]
    fn test_load_empty_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let snap = load_graph(tmp.path().to_str().unwrap());
        assert_eq!(snap.file_count, 0);
        assert!(snap.files.is_empty());
    }

    #[test]
    fn test_load_fixture_graph() {
        let snap = load_graph(&fixture_path());
        // Fixture has 5 .md files
        assert_eq!(snap.file_count, 5);
        // Verify specific files loaded
        assert!(snap.files.contains_key("technical/context-engineering.md"));
        assert!(snap.files.contains_key("business/business-model.md"));
        assert!(snap.files.contains_key("methodology/taste-operationalization.md"));
        assert!(snap.files.contains_key("technical/agentic-systems.md"));
        assert!(snap.files.contains_key("emergent/new-concept.md"));
    }

    #[test]
    fn test_load_skips_non_md() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("good.md"), "---\ntitle: Good\n---\n# Good").unwrap();
        std::fs::write(tmp.path().join("bad.txt"), "Not markdown").unwrap();
        std::fs::write(tmp.path().join("bad.rs"), "fn main() {}").unwrap();
        let snap = load_graph(tmp.path().to_str().unwrap());
        assert_eq!(snap.file_count, 1);
    }

    #[test]
    fn test_load_skips_dotdirs() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("good.md"), "# Good").unwrap();
        let git_dir = tmp.path().join(".git");
        std::fs::create_dir_all(&git_dir).unwrap();
        std::fs::write(git_dir.join("hidden.md"), "# Hidden").unwrap();
        let snap = load_graph(tmp.path().to_str().unwrap());
        assert_eq!(snap.file_count, 1);
    }

    // --- index building ---

    #[test]
    fn test_outbound_links() {
        let snap = load_graph(&fixture_path());
        let ce_links = &snap.index.links["context-engineering"];
        // context-engineering.md links to: taste-operationalization, agentic-systems, business-model
        assert!(ce_links.outbound.contains(&"taste-operationalization".to_string()));
        assert!(ce_links.outbound.contains(&"agentic-systems".to_string()));
        assert!(ce_links.outbound.contains(&"business-model".to_string()));
    }

    #[test]
    fn test_inbound_links() {
        let snap = load_graph(&fixture_path());
        let ce_links = &snap.index.links["context-engineering"];
        // context-engineering should have inbound from: agentic-systems, taste-operationalization, business-model
        assert!(ce_links.inbound.contains(&"agentic-systems".to_string()));
        assert!(ce_links.inbound.contains(&"taste-operationalization".to_string()));
        assert!(ce_links.inbound.contains(&"business-model".to_string()));
    }

    #[test]
    fn test_tree_index() {
        let snap = load_graph(&fixture_path());
        assert!(snap.index.tree.contains_key("technical"));
        let tech_files = &snap.index.tree["technical"];
        assert!(tech_files.contains(&"technical/context-engineering.md".to_string()));
        assert!(tech_files.contains(&"technical/agentic-systems.md".to_string()));
    }

    #[test]
    fn test_domain_index() {
        let snap = load_graph(&fixture_path());
        assert!(snap.index.domains.contains_key("technical"));
        assert!(snap.index.domains.contains_key("business"));
        assert!(snap.index.domains.contains_key("methodology"));
        assert_eq!(snap.index.domains["technical"].len(), 2);
    }

    #[test]
    fn test_tag_index() {
        let snap = load_graph(&fixture_path());
        assert!(snap.index.tags.contains_key("context-engineering"));
        // context-engineering tag appears on: context-engineering.md and business-model.md
        assert!(snap.index.tags["context-engineering"].len() >= 2);
    }

    #[test]
    fn test_status_index() {
        let snap = load_graph(&fixture_path());
        assert!(snap.index.statuses.contains_key("canonical"));
        assert!(snap.index.statuses.contains_key("emergent"));
        assert!(snap.index.statuses.contains_key("validated"));
    }

    #[test]
    fn test_orphan_node() {
        let snap = load_graph(&fixture_path());
        // new-concept has no related_concepts (empty array)
        let links = &snap.index.links["new-concept"];
        assert!(links.outbound.is_empty());
        assert!(links.inbound.is_empty());
    }
}
