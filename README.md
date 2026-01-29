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

## Install

**Windows (PowerShell):**
```powershell
irm https://install.tastematter.dev/install.ps1 | iex
```

**macOS/Linux:**
```bash
curl -fsSL https://install.tastematter.dev/install.sh | bash
```

## Quick Setup (5 minutes)

```bash
# 1. Index your Claude Code sessions
tastematter parse-sessions --project ~/.claude/projects
tastematter build-chains
tastematter index-files

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

### Query Commands

| Command | What It Does |
|---------|-------------|
| `query flex` | Find files by time range, pattern, or session |
| `query co-access <file>` | Find files that get accessed with this one |
| `query chains` | List your conversation chains |
| `query sessions` | Query session-level data |
| `query search <pattern>` | Search file paths by keyword |

### Index Commands

| Command | What It Does |
|---------|-------------|
| `parse-sessions` | Index Claude Code JSONL files |
| `build-chains` | Build conversation chain graph |
| `index-files` | Build file access index |

### Examples

```bash
# Most accessed files in the last 30 days
tastematter query flex --time 30d --limit 20

# Files matching a pattern
tastematter query flex --files "*auth*" --time 14d

# What files are accessed together with this one?
tastematter query co-access src/main.rs --limit 15

# Recent conversation chains
tastematter query chains --limit 10
```

</details>

---

## How It Works

Tastematter indexes your Claude Code session files and builds:
- **File access history** - Which files were read/written, when, how often
- **Session context** - What files were accessed together
- **Conversation chains** - How sessions link via `leafUuid`
- **Co-access graph** - Implicit file relationships

Data is stored locally at `~/.context-os/context_os_events.db`.

---

## License

The skill and documentation in this repository are free to use.
The tastematter CLI is separately licensed at https://tastematter.dev/terms
