# The Skills System

*Extend agent capabilities without token bloat*

---

Skills are Descartes' answer to tool sprawl. Instead of loading 40+ tool definitions into every prompt (2-5k tokens each), skills are CLI scripts that agents invoke via `bash`—only costing tokens when actually used.

## The Problem with Traditional Tools

Consider a typical MCP server setup:

```
Tool: web-search
  - Description: 200 tokens
  - Parameters: 150 tokens
  - Examples: 300 tokens
Total: 650 tokens × 40 tools = 26,000 tokens

Per message. Every message. Even if unused.
```

That's context window consumed before any actual work happens.

## The Skills Solution

Skills are:
- **CLI scripts** in `~/.descartes/skills/`
- **Invoked via bash** when needed
- **Documented in README** files
- **Zero cost** until used (~50 tokens per invocation)

```
Skill: web-search
  Invocation: bash "web-search 'query' 5"
  Cost: ~50 tokens when used
  Cost when unused: 0 tokens
```

---

## Anatomy of a Skill

### Directory Structure

```
~/.descartes/skills/
├── web-search/
│   ├── web-search           # Executable script
│   └── README.md            # Documentation for agent
├── image-gen/
│   ├── image-gen
│   └── README.md
└── code-review/
    ├── code-review
    └── README.md
```

### The Script

```bash
#!/bin/bash
# ~/.descartes/skills/web-search/web-search

QUERY="$1"
MAX_RESULTS="${2:-5}"

# Call your search API
curl -s "https://api.search.example.com/search?q=$QUERY&limit=$MAX_RESULTS" \
  | jq -r '.results[] | "- \(.title): \(.url)"'
```

### The README

```markdown
# web-search

Search the web for information.

## Usage

```bash
web-search "query string" [max_results]
```

## Arguments

- `query`: The search query (required)
- `max_results`: Maximum results to return (default: 5)

## Examples

```bash
web-search "rust async programming"
web-search "kubernetes best practices" 10
```

## Output

Returns markdown-formatted list of results:
```
- Title 1: https://example.com/1
- Title 2: https://example.com/2
```
```

---

## How Agents Discover Skills

When an agent needs capabilities beyond the core 4 tools, it can:

1. **Read the skills directory**
   ```bash
   ls ~/.descartes/skills/
   ```

2. **Check a skill's documentation**
   ```bash
   cat ~/.descartes/skills/web-search/README.md
   ```

3. **Invoke the skill**
   ```bash
   web-search "rust error handling patterns" 3
   ```

### Agent Behavior

The system prompt informs agents about skills:

```
You have access to skills in ~/.descartes/skills/.
Each skill is a CLI tool with a README.md explaining its usage.
To use a skill, invoke it via bash.
```

---

## Creating Custom Skills

### Step 1: Create the Directory

```bash
mkdir -p ~/.descartes/skills/my-skill
```

### Step 2: Write the Script

```bash
#!/bin/bash
# ~/.descartes/skills/my-skill/my-skill

# Your implementation here
echo "Hello from my-skill!"
```

### Step 3: Make Executable

```bash
chmod +x ~/.descartes/skills/my-skill/my-skill
```

### Step 4: Add to PATH (Optional)

For convenience, add to your PATH:

```bash
export PATH="$PATH:$HOME/.descartes/skills/my-skill"
```

Or configure in Descartes:

```toml
# ~/.descartes/config.toml
[skills]
path = ["~/.descartes/skills"]
```

### Step 5: Document It

```markdown
<!-- ~/.descartes/skills/my-skill/README.md -->

# my-skill

Brief description of what this skill does.

## Usage

\`\`\`bash
my-skill <arg1> [arg2]
\`\`\`

## Examples

\`\`\`bash
my-skill foo
my-skill foo bar
\`\`\`
```

---

## Example Skills

### Web Search

```bash
#!/bin/bash
# Searches the web using a search API

QUERY="$1"
MAX="${2:-5}"

curl -s "https://api.duckduckgo.com/?q=$QUERY&format=json" \
  | jq -r ".RelatedTopics[:$MAX][] | \"- \(.Text)\""
```

### Code Review

```bash
#!/bin/bash
# Runs static analysis on code

FILE="$1"

echo "## Linting"
eslint "$FILE" 2>&1 || true

echo "## Type Check"
tsc --noEmit "$FILE" 2>&1 || true

echo "## Security"
npm audit --json 2>&1 | jq '.vulnerabilities | keys[]' || true
```

### Database Query

```bash
#!/bin/bash
# Executes read-only database queries

QUERY="$1"
DB="${2:-$DATABASE_URL}"

psql "$DB" -c "$QUERY" --readonly
```

### Screenshot

```bash
#!/bin/bash
# Takes a screenshot of a URL

URL="$1"
OUTPUT="${2:-screenshot.png}"

chromium --headless --screenshot="$OUTPUT" "$URL"
echo "Screenshot saved to $OUTPUT"
```

### Git Stats

```bash
#!/bin/bash
# Shows repository statistics

echo "## Commit Activity (last 30 days)"
git log --since="30 days ago" --oneline | wc -l

echo "## Top Contributors"
git shortlog -sn --since="30 days ago" | head -5

echo "## Files Changed"
git diff --stat HEAD~10
```

---

## Skills vs MCP Servers

| Aspect | Skills | MCP Servers |
|--------|--------|-------------|
| **Token cost (idle)** | 0 | 2,000-5,000 |
| **Token cost (active)** | ~50-100 | ~200-500 |
| **Setup** | Drop-in scripts | Server configuration |
| **Language** | Any (bash, python, etc.) | TypeScript/JavaScript |
| **State** | Stateless | Can be stateful |
| **Best for** | Simple operations | Complex integrations |

### When to Use Skills

- Simple, stateless operations
- Token-conscious environments
- Quick prototyping
- Custom tooling

### When to Use MCP

- Complex, stateful operations
- Rich type definitions needed
- Official integrations
- Multi-step workflows

---

## Project-Specific Skills

Skills can be project-local:

```
my-project/
├── .descartes/
│   └── skills/
│       └── deploy/
│           ├── deploy
│           └── README.md
└── ...
```

Configure precedence:

```toml
# .descartes/config.toml
[skills]
path = [
  ".descartes/skills",      # Project-local first
  "~/.descartes/skills"     # Then global
]
```

---

## Skill Discovery

Agents can dynamically discover available skills:

### List All Skills

```bash
for skill in ~/.descartes/skills/*/; do
  echo "$(basename $skill)"
done
```

### Get Skill Help

```bash
cat ~/.descartes/skills/web-search/README.md
```

### Check If Skill Exists

```bash
if [ -x ~/.descartes/skills/my-skill/my-skill ]; then
  echo "Skill available"
fi
```

---

## Best Practices

### 1. Keep Skills Focused

One skill, one purpose:

```bash
# Good: Single responsibility
web-search "query"

# Bad: Kitchen sink
do-everything --search "query" --analyze --summarize
```

### 2. Provide Clear Output

```bash
#!/bin/bash
# Output is what the agent sees

echo "## Results"
echo ""
# ... structured output ...
```

### 3. Handle Errors Gracefully

```bash
#!/bin/bash

if [ -z "$1" ]; then
  echo "Error: Query required"
  echo "Usage: web-search <query>"
  exit 1
fi
```

### 4. Document Thoroughly

The README is the agent's interface:

```markdown
# skill-name

Clear, one-line description.

## Usage
## Arguments
## Examples
## Output Format
## Error Handling
```

### 5. Use Environment Variables

```bash
#!/bin/bash
# Credentials from environment

API_KEY="${MY_API_KEY:?Error: MY_API_KEY not set}"
curl -H "Authorization: Bearer $API_KEY" ...
```

---

## Skill Templates

### API Wrapper Template

```bash
#!/bin/bash
set -e

API_KEY="${API_KEY:?Error: API_KEY required}"
ENDPOINT="https://api.example.com"

query="$1"
response=$(curl -s -H "Authorization: Bearer $API_KEY" \
  "$ENDPOINT/search?q=$query")

echo "$response" | jq -r '.results[] | "- \(.title)"'
```

### Data Processing Template

```bash
#!/bin/bash
set -e

input="$1"
format="${2:-json}"

case "$format" in
  json) cat "$input" | jq '.' ;;
  csv)  cat "$input" | jq -r '.[] | [.a, .b] | @csv' ;;
  *)    echo "Unknown format: $format" >&2; exit 1 ;;
esac
```

### Validation Template

```bash
#!/bin/bash
set -e

file="$1"
type="${2:-all}"

echo "## Validation Results for $file"
echo ""

if [[ "$type" == "all" || "$type" == "lint" ]]; then
  echo "### Linting"
  eslint "$file" 2>&1 || echo "Issues found"
fi

if [[ "$type" == "all" || "$type" == "types" ]]; then
  echo "### Type Checking"
  tsc --noEmit "$file" 2>&1 || echo "Issues found"
fi
```

---

## Troubleshooting

### "Skill Not Found"

```bash
# Check PATH
echo $PATH | tr ':' '\n' | grep descartes

# Check executable
ls -la ~/.descartes/skills/my-skill/my-skill
```

### "Permission Denied"

```bash
chmod +x ~/.descartes/skills/my-skill/my-skill
```

### "Command Output Truncated"

Skills output is captured; ensure reasonable output size:

```bash
# Limit output
head -100  # First 100 lines
```

---

## Next Steps

- **[GUI Features →](09-gui-features.md)** — Visual skill monitoring
- **[Sub-Agent Tracking →](10-subagent-tracking.md)** — Track skill usage
- **[Advanced Features →](11-advanced-features.md)** — Compose skills in workflows

---

*Extend your agents' capabilities without the bloat.*
