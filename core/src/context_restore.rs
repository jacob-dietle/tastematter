//! Context restoration builders
//!
//! Pure transform functions that take query results as input and return
//! structured context restoration sections. No DB access, no side effects.
//!
//! Also includes filesystem-based project context discovery using multi-pattern
//! glob matching. This discovers specs, context packages, memory files, and
//! other project context — whatever naming convention the project uses.
//!
//! NOTE: The multi-pattern discovery feature is at risk of being too noisy.
//! Ship and iterate based on real usage. May need per-project config or
//! smarter filtering in future versions.

use std::collections::HashSet;
use std::path::Path;

use crate::types::*;

// =============================================================================
// PHASE 1a: DB-only builders (pure transforms)
// =============================================================================

/// Derive executive summary from session recency and heat distribution.
///
/// - status: healthy (<3d), warning (3-7d), stale (>7d), unknown (no sessions)
/// - work_tempo: active (>3 sessions/week), cooling (1-3), dormant (<1)
/// - last_meaningful_session: most recent session with >5 files
pub fn build_executive_summary(
    sessions: &SessionQueryResult,
    _heat: &HeatResult,
) -> ExecutiveSummary {
    // Find the most recent session timestamp
    let last_session_ts = sessions.sessions.first().map(|s| s.started_at.clone());

    // Determine status from recency
    let status = match &last_session_ts {
        Some(ts) => {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(ts) {
                let hours_ago = (chrono::Utc::now() - dt.with_timezone(&chrono::Utc)).num_hours();
                if hours_ago < 72 {
                    "healthy"
                } else if hours_ago < 168 {
                    "warning"
                } else {
                    "stale"
                }
            } else {
                "unknown"
            }
        }
        None => "unknown",
    }
    .to_string();

    // Determine work tempo from session frequency
    // Count sessions in the last 7 days
    let now = chrono::Utc::now();
    let recent_count = sessions
        .sessions
        .iter()
        .filter(|s| {
            chrono::DateTime::parse_from_rfc3339(&s.started_at)
                .map(|dt| (now - dt.with_timezone(&chrono::Utc)).num_days() <= 7)
                .unwrap_or(false)
        })
        .count();

    let work_tempo = if recent_count > 3 {
        "active"
    } else if recent_count >= 1 {
        "cooling"
    } else {
        "dormant"
    }
    .to_string();

    // Find last meaningful session (>5 files)
    let last_meaningful_session = sessions
        .sessions
        .iter()
        .find(|s| s.file_count > 5)
        .map(|s| s.started_at.clone());

    ExecutiveSummary {
        one_liner: None, // Phase 2
        status,
        work_tempo,
        last_meaningful_session,
    }
}

/// Group co-access files by anchor into work clusters.
///
/// Each anchor file from flex results becomes a cluster center.
/// access_pattern is classified by count/session ratio quadrant.
pub fn build_work_clusters(
    flex: &QueryResult,
    co_access_results: &[CoAccessResult],
) -> Vec<WorkCluster> {
    let mut clusters = Vec::new();

    // Compute median access count for quadrant classification
    let median_count = if flex.results.is_empty() {
        1
    } else {
        let mut counts: Vec<u32> = flex.results.iter().map(|f| f.access_count).collect();
        counts.sort();
        counts[counts.len() / 2]
    };

    let median_sessions = if flex.results.is_empty() {
        1
    } else {
        let mut sessions: Vec<u32> = flex
            .results
            .iter()
            .filter_map(|f| f.session_count)
            .collect();
        if sessions.is_empty() {
            1
        } else {
            sessions.sort();
            sessions[sessions.len() / 2]
        }
    };

    for co_result in co_access_results {
        if co_result.results.is_empty() {
            continue;
        }

        let anchor = &co_result.query_file;

        // Find the anchor in flex results for classification
        let anchor_flex = flex.results.iter().find(|f| f.file_path == *anchor);
        let anchor_count = anchor_flex.map(|f| f.access_count).unwrap_or(0);
        let anchor_sessions = anchor_flex.and_then(|f| f.session_count).unwrap_or(0);

        let access_pattern = match (
            anchor_count >= median_count,
            anchor_sessions >= median_sessions,
        ) {
            (true, true) => "high_access_high_session",
            (true, false) => "high_access_low_session",
            (false, true) => "low_access_high_session",
            (false, false) => "low_access_low_session",
        }
        .to_string();

        // Collect cluster files: anchor + top co-accessed files
        let mut files = vec![anchor.clone()];
        let avg_pmi: f64 = co_result
            .results
            .iter()
            .take(5)
            .map(|c| {
                files.push(c.file_path.clone());
                c.pmi_score
            })
            .sum::<f64>()
            / co_result.results.len().clamp(1, 5) as f64;

        clusters.push(WorkCluster {
            name: None, // Phase 2
            files,
            pmi_score: avg_pmi,
            interpretation: None, // Phase 2
            access_pattern,
        });
    }

    clusters
}

/// Rank files for suggested reading.
///
/// Priority: project context files > surprise co-access files > high-access files.
/// surprise=true if file appears in co-access but not in primary flex results.
pub fn build_suggested_reads(
    flex: &QueryResult,
    co_access_results: &[CoAccessResult],
    context_files: &[ProjectContextFile],
) -> Vec<SuggestedRead> {
    let mut reads = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    let flex_paths: HashSet<&str> = flex.results.iter().map(|f| f.file_path.as_str()).collect();

    // 1. Project context files (highest priority)
    let mut priority = 1u32;
    for ctx in context_files {
        if seen.insert(ctx.path.clone()) {
            reads.push(SuggestedRead {
                path: ctx.path.clone(),
                reason: None, // Phase 2
                priority,
                surprise: false,
            });
            priority += 1;
        }
        if priority > 5 {
            break;
        }
    }

    // 2. Surprise co-access files (in co-access but NOT in flex)
    for co_result in co_access_results {
        for item in &co_result.results {
            if !flex_paths.contains(item.file_path.as_str()) && seen.insert(item.file_path.clone())
            {
                reads.push(SuggestedRead {
                    path: item.file_path.clone(),
                    reason: None, // Phase 2
                    priority,
                    surprise: true,
                });
                priority += 1;
            }
            if reads.len() >= 20 {
                break;
            }
        }
    }

    // 3. High-access files from flex
    for f in &flex.results {
        if seen.insert(f.file_path.clone()) {
            reads.push(SuggestedRead {
                path: f.file_path.clone(),
                reason: None, // Phase 2
                priority,
                surprise: false,
            });
            priority += 1;
        }
        if reads.len() >= 20 {
            break;
        }
    }

    reads
}

/// Collapse daily timeline buckets into multi-day focus periods.
///
/// Detects attention shifts by comparing file sets across adjacent periods
/// using Jaccard similarity (< 0.3 = shift detected).
pub fn build_timeline(timeline: &TimelineData) -> TimelineSection {
    if timeline.buckets.is_empty() {
        return TimelineSection {
            recent_focus: vec![],
            attention_shift: None,
        };
    }

    // Group consecutive days into focus periods
    // Simple approach: chunk by weeks or natural breaks
    let mut periods: Vec<FocusPeriod> = Vec::new();

    // Collect buckets in chronological order (they come DESC)
    let mut sorted_buckets: Vec<&TimeBucket> = timeline.buckets.iter().collect();
    sorted_buckets.sort_by(|a, b| a.date.cmp(&b.date));

    // Group into periods of up to 7 days
    let mut period_start = 0;
    while period_start < sorted_buckets.len() {
        let period_end = (period_start + 7).min(sorted_buckets.len());
        let period_buckets = &sorted_buckets[period_start..period_end];

        // Collect top files for this period from the timeline file data
        let period_dates: HashSet<&str> = period_buckets.iter().map(|b| b.date.as_str()).collect();
        let mut file_accesses: Vec<(&str, u32)> = timeline
            .files
            .iter()
            .map(|f| {
                let count: u32 = f
                    .buckets
                    .iter()
                    .filter(|(date, _)| period_dates.contains(date.as_str()))
                    .map(|(_, c)| c)
                    .sum();
                (f.file_path.as_str(), count)
            })
            .filter(|(_, count)| *count > 0)
            .collect();
        file_accesses.sort_by(|a, b| b.1.cmp(&a.1));

        let total_access: u32 = period_buckets.iter().map(|b| b.access_count).sum();

        periods.push(FocusPeriod {
            start_date: period_buckets
                .first()
                .map(|b| b.date.clone())
                .unwrap_or_default(),
            end_date: period_buckets
                .last()
                .map(|b| b.date.clone())
                .unwrap_or_default(),
            top_files: file_accesses
                .iter()
                .take(5)
                .map(|(f, _)| f.to_string())
                .collect(),
            access_count: total_access,
        });

        period_start = period_end;
    }

    // Detect attention shift between last two periods using Jaccard similarity
    let attention_shift = if periods.len() >= 2 {
        let prev = &periods[periods.len() - 2];
        let curr = &periods[periods.len() - 1];

        let prev_set: HashSet<&str> = prev.top_files.iter().map(|s| s.as_str()).collect();
        let curr_set: HashSet<&str> = curr.top_files.iter().map(|s| s.as_str()).collect();

        let intersection = prev_set.intersection(&curr_set).count() as f64;
        let union = prev_set.union(&curr_set).count() as f64;
        let jaccard = if union > 0.0 {
            intersection / union
        } else {
            1.0
        };

        if jaccard < 0.3 {
            Some(AttentionShift {
                from_period: format!("{} to {}", prev.start_date, prev.end_date),
                to_period: format!("{} to {}", curr.start_date, curr.end_date),
                jaccard_similarity: jaccard,
                description: format!(
                    "Focus shifted significantly (Jaccard={:.2}). Previous top files differ from current.",
                    jaccard
                ),
            })
        } else {
            None
        }
    } else {
        None
    };

    TimelineSection {
        recent_focus: periods,
        attention_shift,
    }
}

/// Generate deterministic insights from heat data and project context age.
///
/// Three detectors:
/// - stale: project context file >7d old
/// - abandoned: files with last_access >14d but high historical count
/// - surprise: unexpected high-PMI file in co-access
pub fn build_deterministic_insights(
    heat: &HeatResult,
    context_files: &[ProjectContextFile],
) -> Vec<ContextInsight> {
    let mut insights = Vec::new();

    // Detector: abandoned files (high count but cold/cool heat)
    for item in &heat.results {
        if item.count_long >= 10
            && (item.heat_level == HeatLevel::Cold || item.heat_level == HeatLevel::Cool)
        {
            insights.push(ContextInsight {
                insight_type: "abandoned".to_string(),
                title: format!("Previously active file now {}", item.heat_level),
                description: format!(
                    "{} has {} total accesses but is now {}. Last accessed: {}",
                    item.file_path, item.count_long, item.heat_level, item.last_access
                ),
                evidence: vec![
                    format!(
                        "count_long={}, heat_level={}",
                        item.count_long, item.heat_level
                    ),
                    format!("last_access={}", item.last_access),
                ],
            });
        }
        if insights.len() >= 5 {
            break;
        }
    }

    // Detector: stale project context (no recent matching context files found)
    if context_files.is_empty() {
        insights.push(ContextInsight {
            insight_type: "stale".to_string(),
            title: "No project context files found".to_string(),
            description: "No specs, context packages, or memory files were discovered for this query. Consider creating documentation.".to_string(),
            evidence: vec!["discovery returned 0 files".to_string()],
        });
    }

    insights
}

/// Build verification metadata from all query result counts.
pub fn build_verification(
    receipt_id: &str,
    flex: &QueryResult,
    sessions: &SessionQueryResult,
    co_access_results: &[CoAccessResult],
) -> ContextVerification {
    let co_access_pairs: u32 = co_access_results
        .iter()
        .map(|c| c.results.len() as u32)
        .sum();

    ContextVerification {
        receipt_id: receipt_id.to_string(),
        files_analyzed: flex.result_count as u32,
        sessions_analyzed: sessions.sessions.len() as u32,
        co_access_pairs,
    }
}

// =============================================================================
// PHASE 1b: Filesystem-based project context discovery
// =============================================================================
//
// RISK: This feature is at risk of being too noisy. It casts a wide net
// across multiple conventional patterns (specs, docs, memory, context packages).
// May need per-project config or smarter filtering. Ship and iterate.

/// Directories to skip during filesystem traversal (performance + noise reduction).
const SKIP_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    "target",
    "__pycache__",
    ".venv",
    "venv",
    "dist",
    "build",
    ".next",
    ".nuxt",
];

/// Classify a file path into a discovery tier based on its directory context.
/// Returns None if the file doesn't match any known pattern.
fn classify_tier(path: &Path) -> Option<&'static str> {
    let path_str = path.to_string_lossy().to_lowercase();
    let path_fwd = path_str.replace('\\', "/");

    // Check file name first for specific named files
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        let name_lower = name.to_lowercase();
        match name_lower.as_str() {
            "claude.md" => return Some("high"),
            "agents.md" | "context.md" => return Some("low"),
            _ => {
                if name_lower.ends_with("_spec.md") {
                    return Some("high");
                }
            }
        }
    }

    // Check directory context
    if path_fwd.contains("/specs/") || path_fwd.contains("/context_packages/") {
        return Some("high");
    }
    if path_fwd.contains("/.claude/memory/") {
        return Some("high");
    }
    if path_fwd.contains("/_system/state/") {
        return Some("medium");
    }
    if path_fwd.contains("/docs/") {
        return Some("medium");
    }
    if path_fwd.contains("/session-memory/") || path_fwd.contains("/session_memory/") {
        return Some("medium");
    }
    if path_fwd.contains("/.cursor/rules/") {
        return Some("low");
    }

    None
}

/// Discover project context files using walkdir with directory filtering.
///
/// Searches from `base_dir` using well-known directory/filename patterns for
/// specs, docs, context packages, memory files, etc. Skips node_modules, .git,
/// target, and other heavy directories. Filters results by query string.
/// Returns files sorted by tier (high > medium > low) then path.
///
/// Cap: 50 files max to prevent runaway output.
pub fn discover_project_context(query: &str, base_dir: &Path) -> Vec<ProjectContextFile> {
    let query_lower = query.to_lowercase();
    let mut results: Vec<ProjectContextFile> = Vec::new();

    let walker = walkdir::WalkDir::new(base_dir)
        .max_depth(8)
        .into_iter()
        .filter_entry(|e| {
            // Skip known heavy directories
            if e.file_type().is_dir() {
                if let Some(name) = e.file_name().to_str() {
                    return !SKIP_DIRS.contains(&name);
                }
            }
            true
        });

    for entry in walker.flatten() {
        if results.len() >= 50 {
            break;
        }

        if !entry.file_type().is_file() {
            continue;
        }

        let path = entry.path();

        // Must be .md or .yaml
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext != "md" && ext != "yaml" && ext != "yml" {
            continue;
        }

        // Classify into a tier (None = not a context file)
        let tier = match classify_tier(path) {
            Some(t) => t,
            None => continue,
        };

        // Filter by query: path must contain the query string
        let path_lower = path.to_string_lossy().to_lowercase();
        if !query_lower.is_empty() && !path_lower.contains(&query_lower) {
            continue;
        }

        // Read and parse
        if let Ok(content) = std::fs::read_to_string(path) {
            let relative = path
                .strip_prefix(base_dir)
                .unwrap_or(path)
                .to_string_lossy()
                .to_string();

            let parsed = parse_context_file(&content);

            results.push(ProjectContextFile {
                path: relative,
                title: parsed.0,
                sections: parsed.1,
                pending_items: parsed.2,
                code_blocks: parsed.3,
                content: if content.len() > 4096 {
                    let mut end = 4096;
                    while !content.is_char_boundary(end) {
                        end -= 1;
                    }
                    content[..end].to_string()
                } else {
                    content
                },
                tier: tier.to_string(),
            });
        }
    }

    // Sort: high tier first, then medium, then low, then by path
    results.sort_by(|a, b| {
        let tier_ord = |t: &str| -> u8 {
            match t {
                "high" => 0,
                "medium" => 1,
                _ => 2,
            }
        };
        tier_ord(&a.tier)
            .cmp(&tier_ord(&b.tier))
            .then_with(|| a.path.cmp(&b.path))
    });

    results
}

/// Parse a context file (markdown or yaml) into structured components.
///
/// Returns: (title, section_headings, pending_items, code_blocks)
fn parse_context_file(content: &str) -> (Option<String>, Vec<String>, Vec<String>, Vec<String>) {
    let mut title: Option<String> = None;
    let mut sections: Vec<String> = Vec::new();
    let mut pending_items: Vec<String> = Vec::new();
    let mut code_blocks: Vec<String> = Vec::new();

    let mut in_code_block = false;
    let mut current_code = String::new();
    let mut is_actionable_code = false;

    for line in content.lines() {
        // Track code fences
        if line.trim_start().starts_with("```") {
            if in_code_block {
                // Closing fence
                if is_actionable_code && !current_code.trim().is_empty() {
                    code_blocks.push(current_code.trim().to_string());
                }
                current_code.clear();
                in_code_block = false;
                is_actionable_code = false;
            } else {
                in_code_block = true;
            }
            continue;
        }

        if in_code_block {
            current_code.push_str(line);
            current_code.push('\n');
            continue;
        }

        // Extract title (first # heading)
        if title.is_none() && line.starts_with("# ") {
            title = Some(line.trim_start_matches("# ").trim().to_string());
            continue;
        }

        // Collect section headings
        if line.starts_with("## ") || line.starts_with("### ") {
            let heading = line.trim_start_matches('#').trim().to_string();
            // Check if this heading precedes actionable code
            let heading_lower = heading.to_lowercase();
            if heading_lower.contains("test")
                || heading_lower.contains("verif")
                || heading_lower.contains("run")
                || heading_lower.contains("quick start")
                || heading_lower.contains("command")
            {
                is_actionable_code = true;
            }
            sections.push(heading);
            continue;
        }

        // Collect pending items (unchecked checkboxes)
        let trimmed = line.trim();
        if trimmed.starts_with("- [ ]") || trimmed.starts_with("* [ ]") {
            let item = trimmed
                .trim_start_matches("- [ ]")
                .trim_start_matches("* [ ]")
                .trim()
                .to_string();
            if !item.is_empty() {
                pending_items.push(item);
            }
        }
    }

    (title, sections, pending_items, code_blocks)
}

/// Build current state from discovered project context and flex data.
///
/// Returns None if no context files were discovered.
pub fn build_current_state(
    context_files: &[ProjectContextFile],
    flex: &QueryResult,
) -> Option<CurrentState> {
    if context_files.is_empty() {
        return None;
    }

    // Build key metrics from flex data
    let metrics = serde_json::json!({
        "files_in_scope": flex.result_count,
        "context_files_found": context_files.len(),
        "tiers": {
            "high": context_files.iter().filter(|f| f.tier == "high").count(),
            "medium": context_files.iter().filter(|f| f.tier == "medium").count(),
            "low": context_files.iter().filter(|f| f.tier == "low").count(),
        }
    });

    // Build evidence from context file titles/paths
    let evidence: Vec<Evidence> = context_files
        .iter()
        .take(5)
        .map(|f| Evidence {
            source: f.path.clone(),
            content: f.title.clone().unwrap_or_else(|| "(untitled)".to_string()),
        })
        .collect();

    Some(CurrentState {
        narrative: None, // Phase 2
        key_metrics: metrics,
        evidence,
    })
}

/// Build continuity from discovered project context and chain data.
///
/// Returns None if no context files with pending items exist.
pub fn build_continuity(
    context_files: &[ProjectContextFile],
    chains: &ChainQueryResult,
) -> Option<Continuity> {
    // Collect pending items from all context files
    let pending_items: Vec<PendingItem> = context_files
        .iter()
        .flat_map(|f| {
            f.pending_items.iter().map(|item| PendingItem {
                text: item.clone(),
                source: f.path.clone(),
            })
        })
        .take(10)
        .collect();

    // Find the most relevant chain
    let chain_context = chains.chains.first().map(|c| ChainContext {
        chain_id: c.chain_id.clone(),
        display_name: c.display_name.clone(),
        session_count: c.session_count,
    });

    // Find what was left off at (most recent high-tier context file)
    let left_off_at = context_files
        .iter()
        .find(|f| f.tier == "high")
        .map(|f| LeftOffAt {
            file: f.path.clone(),
            section: f.sections.last().cloned(),
            timestamp: None,
        });

    if pending_items.is_empty() && chain_context.is_none() && left_off_at.is_none() {
        return None;
    }

    Some(Continuity {
        left_off_at,
        pending_items,
        chain_context,
    })
}

/// Build quick start commands from discovered project context.
///
/// Returns None if no actionable code blocks were found.
pub fn build_quick_start(context_files: &[ProjectContextFile]) -> Option<QuickStart> {
    let commands: Vec<QuickStartCommand> = context_files
        .iter()
        .flat_map(|f| {
            f.code_blocks.iter().map(|block| QuickStartCommand {
                command: block.clone(),
                description: format!("From: {}", f.path),
            })
        })
        .take(5)
        .collect();

    if commands.is_empty() {
        None
    } else {
        Some(QuickStart { commands })
    }
}
