---
title: "Tastematter Context Package 32"
package_number: 32
date: 2026-01-29
status: current
previous_package: "[[31_2026-01-29_PUBLIC_REPO_AND_COPY_REWRITE]]"
related:
  - "[[apps/tastematter/public-repo/README.md]]"
  - "[[apps/tastematter/website/index.html]]"
  - "[[apps/tastematter/website/styles.css]]"
tags:
  - context-package
  - tastematter
  - website
  - mobile
  - visual-design
---

# Tastematter - Context Package 32

## Executive Summary

Deployed website with new "HOW_IT_WORKS" information hierarchy visual (stacked boxes pyramid). Fixed extensive mobile overflow issues caused by `width: calc(100% + 1.5rem)` hack. Website now responsive on mobile.

## Global Context

**Project:** Tastematter - Context intelligence CLI for Claude Code
**Focus This Session:** Information hierarchy visual + mobile responsive fixes

### What Was Accomplished

1. **Information Hierarchy Visual Added:**
   - ASCII art version added to README.md (after "See It In Action")
   - HTML/CSS version added to website (between SEE_IT_IN_ACTION and WHAT_YOU_GET)
   - Shows 3 layers: WORKSTREAMS → SESSIONS → JSONL DATA
   - Pyramid sizing: 60% (top) → 80% (middle) → 100% (bottom)
   - Bidirectional arrows with "TASTEMATTER" labels
   - [VERIFIED: [[apps/tastematter/public-repo/README.md]]:52-89]
   - [VERIFIED: [[apps/tastematter/website/index.html]]:168-231]

2. **README Pushed to GitHub:**
   - Added "How It Works" section with ASCII diagram
   - Commit: "Add information hierarchy diagram (How It Works section)"
   - [VERIFIED: pushed to https://github.com/tastesystems/tastematter]

3. **Mobile Overflow Fixes (Major):**
   - Root cause: `width: calc(100% + 1.5rem)` with `margin: 0 -0.75rem` on `.terminal` and `.code-block`
   - This made elements wider than viewport, causing horizontal scroll
   - Fixed by removing the calc hack, using `width: 100%; max-width: 100%; box-sizing: border-box;`
   - Changed `.terminal-line` from `flex-direction: column` to `display: block` for natural text wrapping
   - Made `.terminal-command` and `.terminal-comment` `display: inline` so text flows
   - Added `word-break: break-all; overflow-wrap: anywhere;` for long URLs
   - [VERIFIED: [[apps/tastematter/website/styles.css]]:683-726]

4. **Website Deployed:**
   - Multiple deploys during mobile fix iterations
   - Final deploy includes all fixes
   - [VERIFIED: live at https://tastematter.dev]

## Local Problem Set

### Completed This Session

- [x] Add ASCII hierarchy diagram to README [VERIFIED: git pushed]
- [x] Add HTML hierarchy diagram to website [VERIFIED: deployed]
- [x] Fix mobile overflow on terminals [VERIFIED: removed calc hack]
- [x] Fix mobile overflow on code blocks [VERIFIED: removed calc hack]
- [x] Add `.terminal-indent` class for demo conversation [VERIFIED: styles.css:880-893]
- [x] Add `.terminal-demo` class for desktop font-size [VERIFIED: styles.css:885-888]
- [x] Add hierarchy box classes (`.hierarchy-top/mid/bottom`) [VERIFIED: styles.css:840-878]
- [x] Deploy website to Cloudflare [VERIFIED: multiple deploys]

### In Progress

- [ ] Mobile centering may still need adjustment
  - User reported "not centered" after aggressive word-break fixes
  - Applied softer word-break: break-word, added centering rules
  - May need further testing on actual mobile device

### Jobs To Be Done (Next Session)

1. [ ] Test website on actual mobile device (not just responsive mode)
2. [ ] Create Discord/Slack for test group
3. [ ] Recruit 10-20 beta testers (per beta-launch-plan.md)
4. [ ] Verify PostHog telemetry events in dashboard

## Technical Details

### Root Cause of Mobile Overflow

The original mobile CSS used a "full-bleed" pattern:
```css
.terminal {
  margin: 0 -0.75rem;           /* Pull left/right */
  width: calc(100% + 1.5rem);   /* Wider than container */
}
```

This was meant to make terminals span full viewport width, but:
- If content inside doesn't wrap, the element overflows
- Negative margin doesn't prevent internal content from pushing wider
- Combined with monospace fonts that don't break naturally = overflow

### The Fix

```css
.terminal {
  width: 100%;
  max-width: 100%;
  box-sizing: border-box;
}

.terminal-line {
  display: block;  /* Not flex, so text wraps naturally */
}

.terminal-command, .terminal-comment {
  display: inline;  /* Flows with text */
  word-break: break-all;
  overflow-wrap: anywhere;
}
```

### CSS Classes Added

| Class | Purpose | Where Used |
|-------|---------|------------|
| `.terminal-indent` | 2rem left padding (0.5rem on mobile) | Demo conversation indented lines |
| `.terminal-demo` | 0.9rem font-size (desktop only) | Demo conversation terminal body |
| `.hierarchy-top` | 60% width (65% mobile) | Workstreams box |
| `.hierarchy-mid` | 80% width (82% mobile) | Sessions box |
| `.hierarchy-bottom` | 100% width | JSONL Data box |
| `.hierarchy-tag` | Tag styling in boxes | "auth feature", "Mon", etc. |
| `.hierarchy-arrow` | Arrow connector styling | ▲ ▼ TASTEMATTER elements |

## File Locations

| File | Purpose | Status |
|------|---------|--------|
| [[apps/tastematter/public-repo/README.md]] | ASCII hierarchy diagram | UPDATED |
| [[apps/tastematter/website/index.html]] | HTML hierarchy section | UPDATED |
| [[apps/tastematter/website/styles.css]] | Mobile fixes + hierarchy styles | UPDATED |

## For Next Agent

**Context Chain:**
- Previous: [[31_2026-01-29_PUBLIC_REPO_AND_COPY_REWRITE]] (public repo, PAS copy)
- This package: Hierarchy visual + mobile fixes
- Next action: Test on actual mobile, then beta recruitment

**Start here:**
1. Open https://tastematter.dev on mobile device
2. Check all sections scroll without horizontal overflow
3. If issues remain, inspect which element overflows with dev tools

**Do NOT:**
- Reintroduce `width: calc(100% + 1.5rem)` pattern
- Use `flex-direction: column` on terminal-line (breaks wrapping)
- Add inline styles that override mobile CSS

**Key insight:**
Mobile overflow was caused by a "full-bleed" CSS pattern that assumes content wraps. Monospace fonts with long URLs don't wrap naturally, so the calc hack just made things wider than viewport. Fix is to use `width: 100%` + `word-break: break-all`.
[VERIFIED: [[apps/tastematter/website/styles.css]]:683-726]
