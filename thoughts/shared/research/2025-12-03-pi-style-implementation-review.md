---
date: 2025-12-03T16:40:00-08:00
researcher: Claude
git_commit: 9dec527e1a541e6cde651cc9c2c0442137f3bdd6
branch: master
repository: cap
topic: "Pi-Style Implementation Review and README Update"
tags: [research, codebase, descartes, tools, transcripts, skills]
status: complete
last_updated: 2025-12-03
last_updated_by: Claude
---

# Research: Pi-Style Implementation Review and README Update

**Date**: 2025-12-03T16:40:00-08:00
**Researcher**: Claude
**Git Commit**: 9dec527e1a541e6cde651cc9c2c0442137f3bdd6
**Branch**: master
**Repository**: cap

## Research Question
Review the full Pi-style implementation and ensure the README reflects what was actually built.

## Summary

The Pi-style CLI-by-default implementation is complete. The README was significantly outdated (described "Phase 1: Foundation" without any mention of the new tools system) and has been fully rewritten to accurately document the implementation.

## Implementation Overview

### What Was Built

1. **Tools Module** (`descartes/core/src/tools/`)
   - `definitions.rs` - 5 tool definitions: read, write, edit, bash, spawn_session
   - `registry.rs` - ToolLevel enum (Minimal, Orchestrator, ReadOnly) and system prompts
   - `executors.rs` - Tool execution implementations with ToolResult return type
   - `mod.rs` - Module exports

2. **Session Transcripts** (`descartes/core/src/session_transcript.rs`)
   - TranscriptEntry, TranscriptMetadata, TranscriptWriter structs
   - JSON format with metadata and entries
   - Default location: `.scud/sessions/` or `~/.descartes/sessions/`
   - Filename format: `YYYY-MM-DD-HH-MM-SS-{short_id}.json`

3. **CLI Spawn Updates** (`descartes/cli/src/commands/spawn.rs`)
   - New flags: `--tool-level`, `--no-spawn`, `--transcript-dir`
   - Tool level parsing with recursive prevention
   - Transcript initialization and saving
   - Stdin piping support

4. **Skills Documentation** (`descartes/docs/SKILLS.md`)
   - Pattern for CLI tools as skills vs MCP servers
   - Context cost comparison
   - Best practices and examples

5. **Example Skill** (`descartes/examples/skills/web-search/`)
   - Executable bash script
   - README with usage documentation

## Code References

### Tools Module
- `descartes/core/src/tools/mod.rs` - Module entry point
- `descartes/core/src/tools/definitions.rs:12-44` - read_tool()
- `descartes/core/src/tools/definitions.rs:48-73` - write_tool()
- `descartes/core/src/tools/definitions.rs:77-113` - edit_tool()
- `descartes/core/src/tools/definitions.rs:117-142` - bash_tool()
- `descartes/core/src/tools/definitions.rs:146-187` - spawn_session_tool()
- `descartes/core/src/tools/registry.rs:10-21` - ToolLevel enum
- `descartes/core/src/tools/registry.rs:24-39` - get_tools()
- `descartes/core/src/tools/registry.rs:43-59` - minimal_system_prompt()
- `descartes/core/src/tools/registry.rs:62-80` - orchestrator_system_prompt()
- `descartes/core/src/tools/executors.rs:27-103` - execute_read()
- `descartes/core/src/tools/executors.rs:106-174` - execute_write()
- `descartes/core/src/tools/executors.rs:177-280` - execute_edit()
- `descartes/core/src/tools/executors.rs:283-353` - execute_bash()
- `descartes/core/src/tools/executors.rs:357-449` - execute_spawn_session()

### Session Transcripts
- `descartes/core/src/session_transcript.rs:14-23` - TranscriptEntry struct
- `descartes/core/src/session_transcript.rs:26-39` - TranscriptMetadata struct
- `descartes/core/src/session_transcript.rs:42-46` - TranscriptWriter struct
- `descartes/core/src/session_transcript.rs:58-98` - TranscriptWriter::new()
- `descartes/core/src/session_transcript.rs:162-179` - TranscriptWriter::save()
- `descartes/core/src/session_transcript.rs:198-207` - default_sessions_dir()

### CLI Spawn
- `descartes/cli/src/main.rs:50-82` - Spawn command definition
- `descartes/cli/src/commands/spawn.rs:15-32` - parse_tool_level()
- `descartes/cli/src/commands/spawn.rs:34-160` - execute()

### Skills
- `descartes/docs/SKILLS.md` - Full documentation
- `descartes/examples/skills/web-search/web-search` - Example executable
- `descartes/examples/skills/web-search/README.md` - Example docs

## README Changes

The README was completely rewritten from describing "Phase 1: Foundation" to accurately reflecting the Pi-style implementation:

**Before**: 290 lines describing basic ModelBackend trait, provider implementations, and future roadmap phases.

**After**: 287 lines documenting:
- Core philosophy (minimal tools, progressive disclosure, observability, recursive prevention)
- Tool levels (Minimal, Orchestrator, ReadOnly)
- All 5 tools with descriptions
- Spawn command with all flags
- Skills pattern and context cost comparison
- Session transcript format
- Architecture diagram
- Development instructions

## Test Coverage

All tests pass:
- 28 tests in descartes-core (tools + session_transcript)
- 27 tests in descartes-cli
- Example skill is executable and functional

## Architecture Documentation

### Tool Flow
```
spawn command → parse_tool_level() → get_tools(level) → ModelRequest
                                    ↓
                         Model calls tool
                                    ↓
                         execute_tool() → ToolResult
                                    ↓
                         TranscriptWriter.add_tool_call/result()
                                    ↓
                         transcript.save() → JSON file
```

### Recursive Prevention
```
Orchestrator spawn (--tool-level orchestrator)
    ↓
Model calls spawn_session tool
    ↓
execute_spawn_session() adds --no-spawn --tool-level minimal
    ↓
Sub-session cannot call spawn_session (not in Minimal tools)
```

## Open Questions

None - implementation is complete and documented.
