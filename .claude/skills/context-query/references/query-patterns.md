# Query Pattern Reference

## Common Patterns by Intent

| User Intent | Key CLI Commands | Synthesis Focus |
|-------------|------------------|-----------------|
| Client context | search -> file -> session | Work areas, recent activity |
| Recent work | recent -> chains -> session | Time-based grouping |
| File relationships | file -> co-access | Related files, sessions |
| Session details | session -> chains | Files touched, chain context |

## Path Substring Patterns

When exact paths are unknown, use these substring strategies:

| Looking For | Try Pattern |
|-------------|-------------|
| Client work | Client name: "pixee", "nickel" |
| Feature area | Feature name: "social_media", "hubspot" |
| Document type | "ARCHITECTURE", "SCOPE", "SPEC" |
| Code files | ".py", ".ts", "cli" |
| Test files | "test_", "_test.py" |

## Interpreting Results

### Access Counts
- 5+ accesses = actively worked on
- 2-4 accesses = touched but not focus
- 1 access = briefly referenced

### Session Patterns
- Multiple sessions on same file = deep work
- Many files in one session = exploration/refactoring
- Created files = new initiative starting

### Chain Context
- Long chains = extended work sessions
- Branched chains = parallel exploration
- Single-session chains = quick tasks

## CLI Command Quick Reference

```bash
# Discovery
context-os query search <pattern>     # Find files by substring
context-os query recent --weeks N     # Weekly activity summary

# File Intelligence
context-os query file <path>          # Sessions that touched file
context-os query co-access <path>     # Files accessed together

# Session Intelligence
context-os query session <id>         # Files touched by session
context-os query chains --limit N     # Conversation chains

# Meta
context-os query log                  # View query history
```

## Example Workflow: "What am I working on for Pixee?"

```bash
# Step 1: Find Pixee files
context-os query search pixee --limit 20
# Result: 147 files, top ones are social_media_automation, hubspot

# Step 2: Drill into top file
context-os query file social_media_automation --limit 5
# Result: 6 sessions, most recent Dec 19

# Step 3: Get session details
context-os query session 61cabe92 --limit 20
# Result: 35 files touched, architecture docs present

# Synthesis:
# - HubSpot mid-funnel automation just started (new architecture doc)
# - Social Media Automation is active focus (6 sessions, recent)
```
