# Tastematter

**Every Claude Code session starts fresh. Your work doesn't.**

---

## The Problem

You've been working on a project for weeks. Hundreds of sessions. Thousands of files touched.

Then you come back after a few days off. Where were you? Which files actually matter? Claude has no idea - it starts every session with a blank slate.

## Sound Familiar?

- **"Where was I?"** - You return to a project and can't remember what you were doing
- **"Which files matter?"** - 50+ files in your codebase, but which ones are you actually working on?
- **"I have to re-explain everything"** - Claude doesn't know your recent context
- **"What else should I look at?"** - Files are related, but you can't see the connections

## Enter Tastematter

Tastematter gives Claude memory of your work.

It indexes your Claude Code sessions and lets you (or Claude) query:
- **Your hot files** - What you've actually been working on
- **File relationships** - Which files get accessed together
- **Work history** - What you did last week, last month
- **Session chains** - How your conversations connect

---

## See It In Action

```
You: What was I working on for the auth feature?

Claude: Let me check your recent activity...

        [runs: tastematter query flex --files "*auth*" --time 14d]

        You've been focused on:
        • src/auth/middleware.ts (12 sessions, 2 days ago)
        • src/auth/jwt.ts (8 sessions)
        • tests/auth.test.ts (6 sessions)

        Your last session touched middleware.ts and jwt.ts together.
        Want me to continue where you left off?
```

---

## How It Works

Your work exists at three levels:

```
┌───────────────────────────────────────┐
│            WORKSTREAMS                │  Your actual projects
│  ┌─────────────┐  ┌─────────────┐     │
│  │ auth        │  │ api         │     │
│  │ feature     │  │ redesign    │     │
│  └─────────────┘  └─────────────┘     │
│           [ COHERENT ]                │
└───────────────────┬───────────────────┘
                    │
               ▲    │    ▼                 TASTEMATTER
                    │
┌───────────────────┴───────────────────┐
│              SESSIONS                 │  Individual conversations
│  ┌─────┐ ┌─────┐ ┌─────┐ ┌─────┐     │
│  │ Mon │ │ Tue │ │ Wed │ │ Thu │     │
│  └─────┘ └─────┘ └─────┘ └─────┘     │
│          [ SCATTERED ]                │
└───────────────────┬───────────────────┘
                    │
               ▲    │    ▼                 TASTEMATTER
                    │
┌───────────────────┴───────────────────┐
│             JSONL DATA                │  Raw Claude Code events
│  ░░░ ░░░ ░░░ ░░░ ░░░ ░░░ ░░░ ░░░     │
│         [ FRAGMENTED ]                │
└───────────────────────────────────────┘
```

**One workstream = many sessions = hundreds of JSONL entries.**

Claude starts fresh every session. It can't see that your "auth feature" work spans Monday, Wednesday, and Friday across dozens of JSONL files.

Tastematter connects the dots. It lets Claude navigate **up** (restore workstream context) and **down** (drill into specific events).

---

## Install

**Windows (PowerShell):**
```powershell
irm https://install.tastematter.dev/install.ps1 | iex
```

**macOS/Linux:**
```bash
curl -fsSL https://install.tastematter.dev/install.sh | bash
```

## Quick Setup

```bash
# 1. Initialize (one command — parses sessions, builds chains, indexes files)
tastematter init

# 2. Query your work
tastematter query flex --time 7d --limit 10
```

That's it. Now Claude (with the skill) or you can query your work patterns.

---

## Want Help Getting Set Up?

Setting up your first Context OS can be tricky. I'll walk you through it.

**[Book a free 15-minute setup call](https://cal.com/jacobdietle/tastematter-cli-setup)**

---

## Claude Code Skill

This repo includes a skill that teaches Claude how to use tastematter.

**To install:**
1. Copy `.claude/skills/context-query/` to your project's `.claude/skills/` directory
2. Claude will automatically use it when you ask about work context

---

## Commands Reference

<details>
<summary>Click to expand full command reference</summary>

### Top-Level Commands

| Command | What It Does |
|---------|-------------|
| `init` | **First-time setup** — parse sessions, build chains, index files |
| `context <query>` | **Start here.** Full context for a topic (summary, heat, clusters, insights) |
| `query <subcommand>` | Targeted queries (flex, heat, chains, etc.) |
| `daemon status` | Show daemon sync state + registration |
| `daemon once` | Run a single sync cycle |

### Query Commands

| Command | What It Does |
|---------|-------------|
| `query flex` | Find files by time range, pattern, or session |
| `query heat` | File heat metrics (specificity, velocity, score, level) |
| `query co-access <file>` | Find files that get accessed with this one |
| `query chains` | List your conversation chains |
| `query sessions` | Query session-level data |
| `query timeline` | Timeline data for visualization |
| `query search <pattern>` | Search file paths by keyword |
| `query verify <receipt>` | Verify a previous query result |

### Index Commands (advanced)

| Command | What It Does |
|---------|-------------|
| `parse-sessions` | Index Claude Code JSONL files |
| `build-chains` | Build conversation chain graph |
| `index-files` | Build file access index |

Use `tastematter init` instead — it runs all three.

### Examples

```bash
# Full context for a project (start here)
tastematter context "auth" --format json

# Most accessed files in the last 30 days
tastematter query flex --time 30d --limit 20

# Files matching a pattern
tastematter query flex --files "*auth*" --time 14d

# File heat metrics
tastematter query heat --time 30d

# What files are accessed together with this one?
tastematter query co-access src/main.rs --limit 15

# Recent conversation chains
tastematter query chains --limit 10
```

</details>

---

## License

The skill and documentation in this repository are free to use.
The tastematter CLI is separately licensed at https://tastematter.dev/terms
