# MCP Publishing Context Packages

Append-only context packages for Tastematter Phase 5: MCP Publishing feature.

## Philosophy

- **Append-only:** Never edit existing packages. New state = new file.
- **Wiki-linked:** Use [[node-name]] for traceable chains.
- **Evidence-based:** Every claim has [VERIFIED/INFERRED/UNVERIFIABLE] attribution.

## Focus

This chain tracks development of:
- Context-as-a-service publishing from Tastematter
- Corpus generation CLI (`tastematter publish corpus`)
- Cloudflare Worker deployment automation
- API key management and authentication
- Query logging and usage analytics

## Canonical Spec

**Primary spec:** [[canonical/10_MCP_PUBLISHING_ARCHITECTURE.md]]

**Proven patterns from:** [[apps/cv_agentic_knowledge/app/deployments/corporate-visions/]]

## Timeline

| # | Date | Description |
|---|------|-------------|
| 00 | 2026-01-17 | Canonical spec complete, proven patterns documented |

## Current State

Latest package: [[00_2026-01-17_MCP_PUBLISHING_SPEC_COMPLETE]]
Status: Spec complete, ready for Phase 5A implementation

## Implementation Phases

### Phase 5A: Internal Publishing (MVP)
- [ ] CLI: Corpus generation (`tastematter publish corpus`)
- [ ] CLI: Deployment (`tastematter publish deploy`)
- [ ] Worker template with auth middleware
- [ ] API key management
- [ ] Config storage (`~/.context-os/publishers.yaml`)

### Phase 5B: Advanced Features (Future)
- [ ] Pay-walling integration (Stripe)
- [ ] Usage analytics dashboard
- [ ] Team features

## How to Use

1. To continue work: Read latest package, follow "Start here" section
2. To understand history: Read packages in order (00 → latest)
3. To add new package: Increment number, never edit existing

## Related Chains

- [[03_current/]] - General Tastematter development
- [[04_daemon/]] - Indexer/daemon investigation
