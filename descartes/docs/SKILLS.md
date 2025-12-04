# Descartes Skills

Skills are CLI tools that agents can invoke via bash, following Pi's philosophy
of progressive disclosure: only load tool definitions when actually needed.

## Why Skills Instead of MCP?

MCP servers inject all tool definitions into every session, consuming 7-15% of
the context window before you start working. Skills are invoked via bash, so the
agent only pays the token cost when it actually uses the tool.

**Context cost comparison:**
| Approach | Context Cost | When Paid |
|----------|-------------|-----------|
| MCP Server | ~2000-5000 tokens per server | Every message |
| Skill (CLI tool) | ~50-100 tokens (usage docs) | Only when used |

## Core Philosophy

From [Pi's blog post](https://marioslab.io/posts/pi/building-a-coding-agent/):

> "If you don't need it, don't build it."

Skills embody this principle:
1. **Minimal tools by default** - read, write, edit, bash
2. **Progressive disclosure** - only discover skill usage when needed
3. **Observability** - all tool use goes through bash, fully visible in transcripts

## Creating a Skill

### 1. Create a CLI tool

```bash
#!/bin/bash
# web-search - Search the web for information
# Usage: web-search --query "search terms" --max-results 5

query="$1"
max_results="${2:-5}"

# Your implementation here
curl -s "https://api.example.com/search?q=${query}&limit=${max_results}" \
  | jq -r '.results[] | "- \(.title): \(.url)"'
```

Make it executable:
```bash
chmod +x web-search
```

### 2. Create a README

The agent reads this when it needs to use the tool:

```markdown
# web-search

Search the web for current information.

## Usage

web-search "your search query" [max_results]

## Arguments

- `query` - The search query (required)
- `max_results` - Maximum results to return (default: 5)

## Examples

# Search for Rust patterns
web-search "rust async patterns 2024"

# Get just one result
web-search "latest npm version" 1

## Output Format

Returns markdown-formatted list of results:
- Title: URL
- Title: URL
...
```

### 3. Tell the Agent About It

In your `AGENTS.md` or system prompt:

```markdown
## Available Skills

You have access to these CLI tools. Read their READMEs for usage:

- `web-search` - Web search (see: /path/to/web-search/README.md)
- `browser` - Browser automation (see: /path/to/browser/README.md)
```

The agent will:
1. See the skill name and brief description
2. Use `read` to check the README when needed
3. Use `bash` to invoke the skill

## Migrating from MCP

If you have an MCP server you want to use as a skill:

### Option 1: Direct CLI Wrapper

Many MCP servers have CLI equivalents. Use those directly.

### Option 2: mcporter (Conceptual)

Create a wrapper script that calls the MCP server:

```bash
#!/bin/bash
# playwright-skill - Browser automation via Playwright MCP

action="$1"
shift

case "$action" in
  navigate)
    # Call MCP server's navigate tool
    echo '{"tool": "navigate", "args": {"url": "'"$1"'"}}' | mcp-client playwright
    ;;
  screenshot)
    echo '{"tool": "screenshot", "args": {"path": "'"$1"'"}}' | mcp-client playwright
    ;;
  *)
    echo "Unknown action: $action"
    echo "Usage: playwright-skill navigate <url>"
    echo "       playwright-skill screenshot <path>"
    exit 1
    ;;
esac
```

### Option 3: Native CLI Alternative

Often there's a native CLI that does what the MCP server does:

| MCP Server | Native CLI Alternative |
|------------|----------------------|
| playwright-mcp | `playwright` CLI |
| puppeteer-mcp | `puppeteer` CLI |
| postgres-mcp | `psql` |
| redis-mcp | `redis-cli` |

## Best Practices

### 1. Self-Documenting Output

Make your skill output clear and parseable:

```bash
# Good - clear structure
echo "Found 3 results:"
echo "1. Result title - https://..."

# Bad - ambiguous
echo "done"
```

### 2. Error Messages

Include actionable error messages:

```bash
if [ -z "$API_KEY" ]; then
  echo "Error: API_KEY environment variable not set"
  echo "Set it with: export API_KEY=your_key"
  exit 1
fi
```

### 3. Minimal Dependencies

Keep skills self-contained:
- Use common tools (curl, jq, grep)
- Avoid requiring complex setup
- Document any required environment variables

### 4. Idempotency

Skills should be safe to retry:
- GET operations are naturally idempotent
- POST operations should handle duplicates gracefully

## Example Skills

See `examples/skills/` for reference implementations:

- `web-search/` - Web search example
- (Add more as needed)

## Comparison with Claude Code Slash Commands

Claude Code uses "skills" loaded from `.claude/commands/`. The concept is similar:
- Both avoid bloating the context with unused tool definitions
- Both use file-based discovery

The difference:
- Claude Code skills are prompt templates
- Descartes skills are executable CLI tools invoked via bash

Both achieve the same goal: progressive disclosure of capabilities.
