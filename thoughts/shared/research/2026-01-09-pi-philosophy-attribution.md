---
date: 2026-01-09T21:21:17Z
researcher: Claude
git_commit: d6d878923d14a2fd98431e12258d8e82267080df
branch: master
repository: cap
topic: "PI Philosophy Attribution - Correcting False Claims in Blog Post"
tags: [research, codebase, descartes, pi-philosophy, attribution]
status: complete
last_updated: 2026-01-09
last_updated_by: Claude
---

# Research: PI Philosophy Attribution - Correcting False Claims in Blog Post

**Date**: 2026-01-09T21:21:17Z
**Researcher**: Claude
**Git Commit**: d6d878923d14a2fd98431e12258d8e82267080df
**Branch**: master
**Repository**: cap

## Research Question

The first Descartes blog post (01-introduction-the-pi-philosophy.md) contained AI-generated claims that falsely took credit for concepts originated by Mario Zechner. The task was to identify these false claims and rewrite the blog post with proper attribution.

## Summary

The original blog post claimed ownership of the "Pi Philosophy" and presented the 4-tool minimalist approach as a Descartes innovation. These concepts were actually created by Mario Zechner in his PI Coding Agent, published November 30, 2025. The blog post was rewritten to properly credit Mario and position Descartes as an implementation of his philosophy with additional features.

## Detailed Findings

### Original False Claims

1. **Line 20 (original)**: "inspired by what we call the **Pi Philosophy**"
   - This language claimed ownership ("we call") of Mario's concept

2. **The 4-tool concept**: Presented as a Descartes innovation
   - Actually originated in Mario Zechner's PI Coding Agent

3. **"Skills" terminology**: Presented as a novel Descartes concept
   - Actually a formalization of Mario's suggestion to use CLI tools via bash instead of MCP servers

### The Actual Source: Mario Zechner's PI Coding Agent

**Source**: https://mariozechner.at/posts/2025-11-30-pi-coding-agent/
**Author**: Mario Zechner
**Published**: November 30, 2025

Key concepts from Mario's original work:
- The 4 core tools: read, write, edit, bash
- Minimal system prompts (<1,000 tokens)
- Rejection of MCP servers in favor of CLI tools
- Full observability through JSON transcripts
- "If I don't need it, it won't be built" philosophy

### What Descartes Actually Contributes

Features that Descartes adds beyond PI:
1. **Multi-provider support** - Anthropic, OpenAI, DeepSeek, Groq, Ollama
2. **Flow Workflow** - Multi-phase PRD-to-code automation
3. **SCUD integration** - Task management with iterative loops
4. **Controlled sub-agent spawning** - One level of delegation only
5. **Pause/Resume** - For long-running tasks
6. **Optional GUI** - Native desktop app for visualization
7. **Skills directory convention** - Formalizing Mario's CLI tool suggestion

### README vs Blog Post Discrepancy

The README.md was already properly attributing Mario:
- Line 29: Quote with link to Pi Philosophy
- Line 489: Proper reference in "References" section

Only the blog post had the false claims.

## Code References

- `descartes/docs/blog/01-introduction-the-pi-philosophy.md` - Rewritten with proper attribution
- `descartes/README.md:29` - Already had proper attribution
- `descartes/README.md:489` - References section with Mario's link

## Changes Made

The blog post was completely rewritten to:
1. Open with "Credit Where It's Due" section crediting Mario
2. Explicitly state "This isn't our idea. It's Mario's. We're just building on it."
3. Rename sections to attribute concepts properly (e.g., "The Pi Philosophy (Mario's Solution)")
4. Clearly separate what Mario created vs what Descartes adds
5. Link to Mario's original post multiple times
6. Remove all "we call" and ownership language

## Related Research

- Mario Zechner's blog: https://mariozechner.at/posts/2025-11-30-pi-coding-agent/

## Open Questions

None - the attribution issue has been resolved.
