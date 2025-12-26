---
date: 2025-12-26T02:02:39Z
researcher: Claude
git_commit: 9e1d025
branch: master
repository: backbone
topic: "OpenCode Server Mode, Headless Operations, and LSP Integration for Descartes Skills"
tags: [research, opencode, lsp, skills, server, headless]
status: complete
last_updated: 2025-12-25
last_updated_by: Claude
---

# Research: OpenCode Server Mode, Headless Operations, and LSP Integration

**Date**: 2025-12-26T02:02:39Z
**Researcher**: Claude
**Git Commit**: 9e1d025
**Branch**: master
**Repository**: backbone

## Research Question

How can we leverage OpenCode's server mode, headless operations, and LSP capabilities to enhance Descartes skills?

## Summary

OpenCode provides a robust client-server architecture with multiple integration points:

1. **Headless Server** (`opencode serve`) - HTTP API with OpenAPI 3.1 spec
2. **Non-interactive CLI** (`opencode run`) - Single-shot commands with attach capability
3. **ACP Server** (`opencode acp`) - Agent Client Protocol via stdin/stdout nd-JSON
4. **LSP Integration** - 24+ language servers with diagnostics exposed to AI
5. **TypeScript SDK** (`@opencode-ai/sdk`) - Programmatic control

For Descartes skills, the most valuable integrations are:
- **`opencode run --attach`** for delegating tasks to OpenCode
- **LSP diagnostics** for code validation feedback
- **Server mode** for persistent connections avoiding cold starts

## Detailed Findings

### 1. OpenCode Server Mode

**Command**: `opencode serve [--port PORT] [--hostname HOST]`

Starts a headless HTTP server exposing:
- OpenAPI 3.1 spec at `/doc`
- REST endpoints for sessions, messages, files, configuration
- Server-Sent Events (SSE) for real-time updates

**Key Endpoints**:
| Category | Purpose |
|----------|---------|
| Sessions | Create, manage, fork conversations |
| Messages | Send prompts, receive responses |
| Files | Search, read, check file status |
| Tools | LSP servers, MCP integration |
| Events | Real-time SSE streaming |

### 2. Non-Interactive CLI (`opencode run`)

**Basic Usage**:
```bash
opencode run "Your prompt here"
opencode run "Query" --format json    # JSON output
opencode run "Query" -q               # Quiet mode for scripts
```

**Attach Mode** (avoids MCP cold starts):
```bash
# Terminal 1: Start server
opencode serve --port 4096

# Terminal 2+: Run commands against server
opencode run "Your prompt" --attach http://localhost:4096
```

**Key Features**:
- Auto-approves all tool permissions in non-interactive mode
- JSON output format for parsing
- Quiet mode (`-q`) suppresses spinners for scripting

### 3. ACP Server Mode

**Command**: `opencode acp`

Starts an Agent Client Protocol server using stdin/stdout with newline-delimited JSON. Useful for:
- IDE plugin integration
- Custom agent orchestration
- Process-based communication

### 4. LSP Integration

OpenCode includes LSP clients for 24+ languages:

| Category | Languages |
|----------|-----------|
| Web | TypeScript, JavaScript, Vue, Svelte, Astro |
| Systems | Rust, Go, C/C++, Zig |
| Backend | Python, PHP, Ruby, Java, Elixir |
| Others | Lua, Clojure, Dart, Gleam, OCaml |

**Current Capability**:
- Full LSP protocol implementation (completions, hover, go-to-definition)
- **Only diagnostics are exposed to the AI assistant**
- Diagnostics wrapped in `<file_diagnostics>` and `<project_diagnostics>` tags

**Configuration** (`opencode.json`):
```json
{
  "lsp": {
    "go": { "disabled": false, "command": "gopls" },
    "rust": { "disabled": false, "command": "rust-analyzer" }
  }
}
```

**Debug Command**:
```bash
opencode debug lsp diagnostics ./path/to/file.ts
```

### 5. TypeScript SDK

**Installation**:
```bash
npm install @opencode-ai/sdk
```

**Usage**:
```typescript
import { createOpencode, createOpencodeClient } from '@opencode-ai/sdk';

// Create server + client
const opencode = await createOpencode({ port: 4096 });

// Or connect to existing server
const client = createOpencodeClient({ baseUrl: 'http://localhost:4096' });

// Send prompt
const response = await client.sessions.prompt(sessionId, {
  message: "Your prompt"
});
```

### 6. Existing Descartes Integration Plans

There's an existing PRD at `.scud/docs/opencode_tui.md` for OpenCode attachment:
- Focus on attaching to paused Descartes agents
- Entry point: `descartes agents attach opencode <agent-id>`
- Environment variables: `DESCARTES_ATTACH_TOKEN`, `DESCARTES_AGENT_SOCKET`

## Integration Opportunities for Descartes Skills

### Option A: OpenCode Run Skill (Simplest)

Create a skill that delegates to `opencode run`:

```bash
#!/bin/bash
# .descartes/skills/opencode-ask
opencode run "$@" --format json -q
```

**Pros**: Simple, immediate value
**Cons**: Cold start on each invocation

### Option B: Persistent Server Skill

Start `opencode serve` and use `--attach`:

```bash
#!/bin/bash
# .descartes/skills/opencode-query
OPENCODE_SERVER="${OPENCODE_SERVER:-http://localhost:4096}"
opencode run "$@" --attach "$OPENCODE_SERVER" --format json -q
```

**Pros**: No cold starts, faster responses
**Cons**: Requires server management

### Option C: LSP Diagnostics Skill

Leverage OpenCode's LSP for code validation:

```bash
#!/bin/bash
# .descartes/skills/lsp-check
opencode debug lsp diagnostics "$1"
```

**Pros**: Direct access to LSP diagnostics
**Cons**: Limited to diagnostics (not completions/hover)

### Option D: SDK Integration (Advanced)

Use `@opencode-ai/sdk` for deeper integration:

```typescript
// Could be a Node.js-based skill
import { createOpencodeClient } from '@opencode-ai/sdk';
const client = createOpencodeClient({ baseUrl: process.env.OPENCODE_SERVER });
// ... programmatic control
```

**Pros**: Full API access, type-safe
**Cons**: Requires Node.js runtime

## Code References

- OpenCode TUI PRD: `.scud/docs/opencode_tui.md`
- Existing skills example: `descartes/examples/skills/web-search/`
- Skills documentation: `descartes/docs/SKILLS.md`
- Doctor check for skills: `descartes/cli/src/commands/doctor.rs:163-184`

## Architecture Documentation

The recommended integration pattern:

```
┌─────────────────┐     ┌──────────────────┐     ┌─────────────────┐
│  Descartes CLI  │────▶│  OpenCode Skill  │────▶│  opencode serve │
│   (spawn)       │     │  (bash wrapper)  │     │  (HTTP server)  │
└─────────────────┘     └──────────────────┘     └─────────────────┘
                                                          │
                                                          ▼
                                                 ┌─────────────────┐
                                                 │   LSP Servers   │
                                                 │  (diagnostics)  │
                                                 └─────────────────┘
```

## Related Research

- `thoughts/shared/research/2025-12-06-provider-support-and-opencode-zen.md`
- `thoughts/shared/plans/2025-12-06-opencode-zen-provider.md`

## Open Questions

1. Should we start `opencode serve` automatically with the Descartes daemon?
2. How to handle authentication/tokens between Descartes and OpenCode?
3. Should we expose OpenCode's full API or just specific capabilities?
4. How to share context between Descartes agents and OpenCode sessions?
