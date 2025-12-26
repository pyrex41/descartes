# Descartes Skills

This directory contains CLI tools that agents can invoke via bash.

## Available Skills

| Skill | Description |
|-------|-------------|
| `opencode-delegate` | Delegate tasks to OpenCode AI |
| `opencode-server` | Manage OpenCode server lifecycle |
| `lsp-check` | Get LSP diagnostics for a file |
| `lsp-fix` | Fix LSP errors using OpenCode |
| `web-search` | Search the web (demo) |

## Usage

Each skill has built-in help:

```bash
.descartes/skills/opencode-delegate --help
.descartes/skills/lsp-check --help
```

## Adding New Skills

See `descartes/docs/SKILLS.md` for the skill creation guide.
