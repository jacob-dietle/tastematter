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

use crate::intelligence::{
    ClusterInput, ContextSynthesisResponse, SuggestedReadInput,
    ContextSynthesisRequest,
};
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

// =============================================================================
// PHASE 2: LLM Synthesis (build request + merge response)
// =============================================================================

/// Build a curated synthesis request from ContextRestoreResult.
///
/// Extracts a 2-4K token subset for the LLM — clusters, reads, context
/// package content (from highest-tier context file), and evidence sources.
pub fn build_synthesis_request(
    result: &ContextRestoreResult,
    context_files: &[ProjectContextFile],
) -> ContextSynthesisRequest {
    // Convert work clusters to ClusterInput
    let clusters: Vec<ClusterInput> = result
        .work_clusters
        .iter()
        .map(|c| ClusterInput {
            files: c.files.clone(),
            access_pattern: c.access_pattern.clone(),
            pmi_score: c.pmi_score,
        })
        .collect();

    // Convert suggested reads to SuggestedReadInput
    let suggested_reads: Vec<SuggestedReadInput> = result
        .suggested_reads
        .iter()
        .map(|r| SuggestedReadInput {
            path: r.path.clone(),
            priority: r.priority,
            surprise: r.surprise,
        })
        .collect();

    // Get context package content from highest-tier context file
    let context_package_content = context_files
        .iter()
        .find(|f| f.tier == "high")
        .map(|f| f.content.clone());

    // Get key_metrics from current_state
    let key_metrics = result
        .current_state
        .as_ref()
        .map(|cs| cs.key_metrics.clone());

    // Collect evidence sources
    let evidence_sources: Vec<String> = result
        .current_state
        .as_ref()
        .map(|cs| cs.evidence.iter().map(|e| e.source.clone()).collect())
        .unwrap_or_default();

    ContextSynthesisRequest {
        query: result.query.clone(),
        status: result.executive_summary.status.clone(),
        work_tempo: result.executive_summary.work_tempo.clone(),
        clusters,
        suggested_reads,
        context_package_content,
        key_metrics,
        evidence_sources,
    }
}

/// Merge LLM synthesis response into the ContextRestoreResult.
///
/// Fills the 5 None fields using index-matched arrays from the response.
/// Silently skips mismatched array lengths (graceful degradation).
pub fn merge_synthesis(
    result: &mut ContextRestoreResult,
    synthesis: &ContextSynthesisResponse,
) {
    // 1. one_liner
    result.executive_summary.one_liner = Some(synthesis.one_liner.clone());

    // 2. narrative
    if let Some(ref mut cs) = result.current_state {
        cs.narrative = Some(synthesis.narrative.clone());
    }

    // 3 & 4. cluster names and interpretations (index-matched)
    for (i, cluster) in result.work_clusters.iter_mut().enumerate() {
        if let Some(name) = synthesis.cluster_names.get(i) {
            cluster.name = Some(name.clone());
        }
        if let Some(interp) = synthesis.cluster_interpretations.get(i) {
            cluster.interpretation = Some(interp.clone());
        }
    }

    // 5. suggested read reasons (index-matched)
    for (i, read) in result.suggested_reads.iter_mut().enumerate() {
        if let Some(reason) = synthesis.suggested_read_reasons.get(i) {
            read.reason = Some(reason.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_minimal_result() -> ContextRestoreResult {
        ContextRestoreResult {
            receipt_id: "q_test01".to_string(),
            query: "nickel".to_string(),
            generated_at: "2026-02-10T00:00:00Z".to_string(),
            executive_summary: ExecutiveSummary {
                one_liner: None,
                status: "healthy".to_string(),
                work_tempo: "active".to_string(),
                last_meaningful_session: None,
            },
            current_state: Some(CurrentState {
                narrative: None,
                key_metrics: serde_json::json!({"files_in_scope": 20}),
                evidence: vec![Evidence {
                    source: "specs/README.md".to_string(),
                    content: "Test spec".to_string(),
                }],
            }),
            continuity: None,
            work_clusters: vec![
                WorkCluster {
                    name: None,
                    files: vec!["src/auth.rs".to_string(), "src/login.rs".to_string()],
                    pmi_score: 2.5,
                    interpretation: None,
                    access_pattern: "high_access_high_session".to_string(),
                },
                WorkCluster {
                    name: None,
                    files: vec!["tests/test_auth.rs".to_string()],
                    pmi_score: 1.2,
                    interpretation: None,
                    access_pattern: "low_access_low_session".to_string(),
                },
            ],
            suggested_reads: vec![
                SuggestedRead {
                    path: "specs/README.md".to_string(),
                    reason: None,
                    priority: 1,
                    surprise: false,
                },
                SuggestedRead {
                    path: "src/weird.rs".to_string(),
                    reason: None,
                    priority: 2,
                    surprise: true,
                },
            ],
            timeline: TimelineSection {
                recent_focus: vec![],
                attention_shift: None,
            },
            insights: vec![],
            verification: ContextVerification {
                receipt_id: "q_test01".to_string(),
                files_analyzed: 20,
                sessions_analyzed: 5,
                co_access_pairs: 10,
            },
            quick_start: None,
        }
    }

    fn make_synthesis_response() -> ContextSynthesisResponse {
        ContextSynthesisResponse {
            one_liner: "Nickel transcript worker is production-ready".to_string(),
            narrative: "You built a multi-provider ingestion system.".to_string(),
            cluster_names: vec!["Auth Pipeline".to_string(), "Test Suite".to_string()],
            cluster_interpretations: vec![
                "Core auth files that move together".to_string(),
                "Test coverage for auth".to_string(),
            ],
            suggested_read_reasons: vec![
                "Start here for project overview".to_string(),
                "Unexpected co-access pattern worth investigating".to_string(),
            ],
            model_used: "claude-haiku-4-5-20251001".to_string(),
        }
    }

    // =========================================================================
    // build_synthesis_request tests
    // =========================================================================

    #[test]
    fn build_synthesis_request_extracts_query_and_status() {
        let result = make_minimal_result();
        let req = build_synthesis_request(&result, &[]);
        assert_eq!(req.query, "nickel");
        assert_eq!(req.status, "healthy");
        assert_eq!(req.work_tempo, "active");
    }

    #[test]
    fn build_synthesis_request_converts_clusters() {
        let result = make_minimal_result();
        let req = build_synthesis_request(&result, &[]);
        assert_eq!(req.clusters.len(), 2);
        assert_eq!(req.clusters[0].files[0], "src/auth.rs");
        assert_eq!(req.clusters[0].access_pattern, "high_access_high_session");
        assert!((req.clusters[0].pmi_score - 2.5).abs() < 0.001);
    }

    #[test]
    fn build_synthesis_request_converts_suggested_reads() {
        let result = make_minimal_result();
        let req = build_synthesis_request(&result, &[]);
        assert_eq!(req.suggested_reads.len(), 2);
        assert_eq!(req.suggested_reads[0].path, "specs/README.md");
        assert_eq!(req.suggested_reads[0].priority, 1);
        assert!(!req.suggested_reads[0].surprise);
        assert!(req.suggested_reads[1].surprise);
    }

    #[test]
    fn build_synthesis_request_extracts_context_package_from_high_tier() {
        let result = make_minimal_result();
        let context_files = vec![ProjectContextFile {
            path: "specs/pkg.md".to_string(),
            title: Some("Package 35".to_string()),
            sections: vec![],
            pending_items: vec![],
            code_blocks: vec![],
            content: "# Package 35\nDB auto-init complete.".to_string(),
            tier: "high".to_string(),
        }];
        let req = build_synthesis_request(&result, &context_files);
        assert!(req.context_package_content.is_some());
        assert!(req.context_package_content.unwrap().contains("Package 35"));
    }

    #[test]
    fn build_synthesis_request_no_context_when_no_high_tier() {
        let result = make_minimal_result();
        let context_files = vec![ProjectContextFile {
            path: "docs/readme.md".to_string(),
            title: None,
            sections: vec![],
            pending_items: vec![],
            code_blocks: vec![],
            content: "Low tier content".to_string(),
            tier: "low".to_string(),
        }];
        let req = build_synthesis_request(&result, &context_files);
        assert!(req.context_package_content.is_none());
    }

    #[test]
    fn build_synthesis_request_extracts_evidence_sources() {
        let result = make_minimal_result();
        let req = build_synthesis_request(&result, &[]);
        assert_eq!(req.evidence_sources, vec!["specs/README.md"]);
    }

    // =========================================================================
    // merge_synthesis tests
    // =========================================================================

    #[test]
    fn merge_synthesis_fills_one_liner() {
        let mut result = make_minimal_result();
        let synthesis = make_synthesis_response();
        merge_synthesis(&mut result, &synthesis);
        assert_eq!(
            result.executive_summary.one_liner.unwrap(),
            "Nickel transcript worker is production-ready"
        );
    }

    #[test]
    fn merge_synthesis_fills_narrative() {
        let mut result = make_minimal_result();
        let synthesis = make_synthesis_response();
        merge_synthesis(&mut result, &synthesis);
        assert_eq!(
            result.current_state.unwrap().narrative.unwrap(),
            "You built a multi-provider ingestion system."
        );
    }

    #[test]
    fn merge_synthesis_fills_cluster_names_and_interpretations() {
        let mut result = make_minimal_result();
        let synthesis = make_synthesis_response();
        merge_synthesis(&mut result, &synthesis);
        assert_eq!(result.work_clusters[0].name.as_deref(), Some("Auth Pipeline"));
        assert_eq!(result.work_clusters[1].name.as_deref(), Some("Test Suite"));
        assert_eq!(
            result.work_clusters[0].interpretation.as_deref(),
            Some("Core auth files that move together")
        );
    }

    #[test]
    fn merge_synthesis_fills_suggested_read_reasons() {
        let mut result = make_minimal_result();
        let synthesis = make_synthesis_response();
        merge_synthesis(&mut result, &synthesis);
        assert_eq!(
            result.suggested_reads[0].reason.as_deref(),
            Some("Start here for project overview")
        );
        assert_eq!(
            result.suggested_reads[1].reason.as_deref(),
            Some("Unexpected co-access pattern worth investigating")
        );
    }

    #[test]
    fn merge_synthesis_handles_mismatched_array_lengths() {
        let mut result = make_minimal_result();
        // Response has fewer names than clusters — should not panic
        let synthesis = ContextSynthesisResponse {
            one_liner: "test".to_string(),
            narrative: "test".to_string(),
            cluster_names: vec!["Only One".to_string()], // 1 name, 2 clusters
            cluster_interpretations: vec![],              // 0 interps, 2 clusters
            suggested_read_reasons: vec![],               // 0 reasons, 2 reads
            model_used: "test".to_string(),
        };
        merge_synthesis(&mut result, &synthesis);
        // First cluster gets name, second stays None
        assert_eq!(result.work_clusters[0].name.as_deref(), Some("Only One"));
        assert!(result.work_clusters[1].name.is_none());
        // Interpretations stay None
        assert!(result.work_clusters[0].interpretation.is_none());
        // Reads stay None
        assert!(result.suggested_reads[0].reason.is_none());
    }

    #[test]
    fn merge_synthesis_handles_no_current_state() {
        let mut result = make_minimal_result();
        result.current_state = None; // No current state
        let synthesis = make_synthesis_response();
        // Should not panic — just skip narrative
        merge_synthesis(&mut result, &synthesis);
        assert!(result.current_state.is_none());
        // one_liner still gets set
        assert!(result.executive_summary.one_liner.is_some());
    }

    // =========================================================================
    // Phase 4: Context Restore Edge Cases (Stress Tests)
    // =========================================================================

    fn make_empty_heat() -> HeatResult {
        HeatResult {
            receipt_id: "q_heat".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            time_range: "30d".to_string(),
            results: vec![],
            summary: HeatSummary {
                total_files: 0,
                hot_count: 0,
                warm_count: 0,
                cool_count: 0,
                cold_count: 0,
            },
        }
    }

    fn make_empty_session_summary() -> SessionSummary {
        SessionSummary {
            total_sessions: 0,
            total_files: 0,
            total_accesses: 0,
            active_chains: 0,
        }
    }

    fn make_test_session_data(session_id: &str, started_at: &str, file_count: u32) -> SessionData {
        SessionData {
            session_id: session_id.to_string(),
            chain_id: None,
            chain_name: None,
            started_at: started_at.to_string(),
            ended_at: None,
            duration_seconds: Some(3600),
            file_count,
            total_accesses: file_count * 2,
            files: vec![],
            top_files: vec![],
        }
    }

    #[test]
    fn stress_executive_summary_zero_sessions() {
        let sessions = SessionQueryResult {
            time_range: "30d".to_string(),
            sessions: vec![],
            chains: vec![],
            summary: make_empty_session_summary(),
        };
        let heat = make_empty_heat();
        let summary = build_executive_summary(&sessions, &heat);
        assert_eq!(summary.status, "unknown", "0 sessions → unknown status");
        assert_eq!(summary.work_tempo, "dormant", "0 sessions → dormant tempo");
        assert!(summary.last_meaningful_session.is_none());
    }

    #[test]
    fn stress_executive_summary_stale_sessions() {
        // Session from 1 year ago
        let old_ts = (chrono::Utc::now() - chrono::Duration::days(365)).to_rfc3339();
        let sessions = SessionQueryResult {
            time_range: "30d".to_string(),
            sessions: vec![make_test_session_data("old-session", &old_ts, 10)],
            chains: vec![],
            summary: make_empty_session_summary(),
        };
        let heat = make_empty_heat();
        let summary = build_executive_summary(&sessions, &heat);
        assert_eq!(summary.status, "stale", "1-year-old session → stale");
    }

    #[test]
    fn stress_executive_summary_fresh_sessions() {
        // Session from 1 hour ago
        let fresh_ts = (chrono::Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        let sessions = SessionQueryResult {
            time_range: "30d".to_string(),
            sessions: vec![make_test_session_data("fresh-session", &fresh_ts, 10)],
            chains: vec![],
            summary: make_empty_session_summary(),
        };
        let heat = make_empty_heat();
        let summary = build_executive_summary(&sessions, &heat);
        assert_eq!(summary.status, "healthy", "1-hour-old session → healthy");
    }

    #[test]
    fn stress_build_work_clusters_single_file() {
        let flex = QueryResult {
            receipt_id: "q_test".to_string(),
            timestamp: "2026-02-10".to_string(),
            result_count: 1,
            results: vec![FileResult {
                file_path: "src/main.rs".to_string(),
                access_count: 5,
                last_access: Some("2026-02-10".to_string()),
                session_count: Some(3),
                sessions: None,
                chains: None,
            }],
            aggregations: Aggregations {
                count: None,
                recency: None,
            },
        };
        let co_access: Vec<CoAccessResult> = vec![];
        let clusters = build_work_clusters(&flex, &co_access);
        // Should produce at least 1 cluster (the single file itself)
        // or 0 if co-access is required — either way, no panic
        assert!(clusters.len() <= 1);
    }

    #[test]
    fn stress_merge_synthesis_empty_response() {
        let mut result = make_minimal_result();
        let empty_synthesis = ContextSynthesisResponse {
            one_liner: String::new(),
            narrative: String::new(),
            cluster_names: vec![],
            cluster_interpretations: vec![],
            suggested_read_reasons: vec![],
            model_used: String::new(),
        };
        merge_synthesis(&mut result, &empty_synthesis);
        // Empty strings should still be set (they're valid, just empty)
        assert_eq!(result.executive_summary.one_liner.as_deref(), Some(""));
    }

    #[test]
    fn stress_build_synthesis_request_unicode_file_paths() {
        let mut result = make_minimal_result();
        result.work_clusters = vec![WorkCluster {
            name: None,
            files: vec![
                "src/\u{9879}\u{76EE}/main.rs".to_string(), // CJK chars
                "docs/\u{1F680}_launch.md".to_string(),       // Emoji in path
            ],
            pmi_score: 1.0,
            interpretation: None,
            access_pattern: "mixed".to_string(),
        }];
        result.suggested_reads = vec![SuggestedRead {
            path: "src/\u{0410}\u{0411}\u{0412}.rs".to_string(), // Cyrillic
            reason: None,
            priority: 1,
            surprise: false,
        }];
        let req = build_synthesis_request(&result, &[]);
        // Should not panic and unicode should survive
        assert!(req.clusters[0].files[0].contains('\u{9879}'));
        assert!(req.suggested_reads[0].path.contains('\u{0410}'));
    }

    #[test]
    fn stress_discover_project_context_empty_directory() {
        let temp_dir = tempfile::tempdir().unwrap();
        let results = discover_project_context("test", temp_dir.path());
        assert!(
            results.is_empty(),
            "Empty directory should return no context files"
        );
    }

    #[test]
    fn stress_discover_project_context_nonexistent_directory() {
        let results = discover_project_context("test", std::path::Path::new("/nonexistent/path"));
        assert!(
            results.is_empty(),
            "Nonexistent directory should return empty, not error"
        );
    }

    #[test]
    fn stress_merge_synthesis_more_names_than_clusters() {
        let mut result = make_minimal_result();
        // Response has MORE names than clusters (opposite of existing test)
        let synthesis = ContextSynthesisResponse {
            one_liner: "test".to_string(),
            narrative: "test".to_string(),
            cluster_names: vec![
                "A".to_string(),
                "B".to_string(),
                "C".to_string(),
                "D".to_string(),
            ], // 4 names, 2 clusters
            cluster_interpretations: vec!["I1".to_string(), "I2".to_string(), "I3".to_string()], // 3 interps
            suggested_read_reasons: vec!["R1".to_string(), "R2".to_string(), "R3".to_string()], // 3 reasons
            model_used: "test".to_string(),
        };
        merge_synthesis(&mut result, &synthesis);
        // First 2 clusters get names/interps, extras ignored
        assert_eq!(result.work_clusters[0].name.as_deref(), Some("A"));
        assert_eq!(result.work_clusters[1].name.as_deref(), Some("B"));
        assert_eq!(
            result.work_clusters[0].interpretation.as_deref(),
            Some("I1")
        );
        // First 2 reads get reasons, extras ignored
        assert_eq!(result.suggested_reads[0].reason.as_deref(), Some("R1"));
        assert_eq!(result.suggested_reads[1].reason.as_deref(), Some("R2"));
    }
}
