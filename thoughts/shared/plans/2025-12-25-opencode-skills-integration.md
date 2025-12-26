# OpenCode Skills Integration Implementation Plan

**Status**: COMPLETE (2025-12-25)

## Overview

Integrate OpenCode's server mode, headless CLI, and LSP capabilities into Descartes as skills, enabling agents to delegate complex tasks to OpenCode and leverage its language server diagnostics.

## Current State Analysis

- OpenCode is installed at `/Users/reuben/.bun/bin/opencode`
- Skills directory exists at `.descartes/skills/` with one example skill (`web-search`)
- Doctor command checks for skills at `.descartes/skills/`
- OpenCode supports:
  - `opencode serve` - Headless HTTP server with OpenAPI spec
  - `opencode run` - Non-interactive CLI with `--attach` for connecting to running servers
  - `opencode debug lsp diagnostics` - Direct LSP diagnostics access
  - 24+ language server integrations (Rust, Go, TypeScript, Python, etc.)

### Key Discoveries:
- OpenCode's `--attach` mode avoids MCP cold starts by connecting to a running server
- LSP diagnostics are the only LSP feature currently exposed to AI assistants
- Skills pattern in Descartes follows Pi's progressive disclosure philosophy
- Existing PRD at `.scud/docs/opencode_tui.md` describes OpenCode attachment for paused agents

## Desired End State

After this plan is complete:
1. **`opencode-delegate` skill** - Delegates prompts to OpenCode with optional attach mode
2. **`opencode-server` skill** - Manages OpenCode server lifecycle (start/stop/status)
3. **`lsp-check` skill** - Runs LSP diagnostics on files
4. **`lsp-fix` skill** - Runs diagnostics and asks OpenCode to fix issues
5. **Documentation** - Updated SKILLS.md with OpenCode skill documentation

### Verification:
```bash
# Skills are available
descartes doctor | grep "skills available"

# Each skill works
.descartes/skills/opencode-server status
.descartes/skills/opencode-delegate "What is 2+2?"
.descartes/skills/lsp-check src/main.rs
```

## What We're NOT Doing

- Automatic startup of OpenCode server with Descartes daemon (future enhancement)
- Full OpenCode SDK integration (TypeScript/Node.js dependency)
- Session sharing between Descartes and OpenCode (complex state management)
- OpenCode TUI attachment for paused agents (separate PRD exists)

## Implementation Approach

Four phases, each building on the previous:
1. Core skills (opencode-delegate, lsp-check)
2. Server management skill (opencode-server)
3. Advanced LSP skill (lsp-fix)
4. Documentation and integration

---

## Phase 1: Core OpenCode Skills

### Overview
Create the foundational skills for delegating to OpenCode and checking LSP diagnostics.

### Changes Required:

#### 1.1 Create opencode-delegate Skill

**File**: `descartes/.descartes/skills/opencode-delegate`

```bash
#!/bin/bash
# opencode-delegate - Delegate a task to OpenCode
#
# Usage: opencode-delegate "prompt" [--attach URL] [--format json|text]
#
# Examples:
#   opencode-delegate "Explain this Rust code" --file src/main.rs
#   opencode-delegate "Fix the compilation errors" --attach http://localhost:4096

set -e

# Parse arguments
PROMPT=""
ATTACH_URL=""
FORMAT="text"
FILES=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --attach)
            ATTACH_URL="$2"
            shift 2
            ;;
        --format)
            FORMAT="$2"
            shift 2
            ;;
        --file)
            FILES="$FILES --file $2"
            shift 2
            ;;
        -h|--help)
            echo "Usage: opencode-delegate \"prompt\" [options]"
            echo ""
            echo "Delegate a task to OpenCode AI assistant."
            echo ""
            echo "Options:"
            echo "  --attach URL    Connect to running OpenCode server (faster)"
            echo "  --format FMT    Output format: text (default) or json"
            echo "  --file PATH     Include file context (can be repeated)"
            echo "  -h, --help      Show this help"
            echo ""
            echo "Examples:"
            echo "  opencode-delegate \"What does this function do?\""
            echo "  opencode-delegate \"Fix errors\" --attach http://localhost:4096"
            echo "  opencode-delegate \"Review this\" --file src/lib.rs --format json"
            exit 0
            ;;
        *)
            if [ -z "$PROMPT" ]; then
                PROMPT="$1"
            else
                PROMPT="$PROMPT $1"
            fi
            shift
            ;;
    esac
done

if [ -z "$PROMPT" ]; then
    echo "Error: No prompt provided"
    echo "Usage: opencode-delegate \"prompt\" [options]"
    exit 1
fi

# Check if opencode is available
if ! command -v opencode &> /dev/null; then
    echo "Error: opencode not found in PATH"
    echo "Install it from: https://opencode.ai"
    exit 1
fi

# Build command
CMD="opencode run"

if [ -n "$ATTACH_URL" ]; then
    CMD="$CMD --attach $ATTACH_URL"
fi

if [ "$FORMAT" = "json" ]; then
    CMD="$CMD --format json"
fi

CMD="$CMD -q"  # Quiet mode for scripting

# Execute
$CMD "$PROMPT"
```

#### 1.2 Create lsp-check Skill

**File**: `descartes/.descartes/skills/lsp-check`

```bash
#!/bin/bash
# lsp-check - Get LSP diagnostics for a file
#
# Usage: lsp-check <file> [--format json|text]
#
# Returns compilation errors, warnings, and hints from the language server.

set -e

FILE="$1"
FORMAT="${2:-text}"

if [ -z "$FILE" ]; then
    echo "Usage: lsp-check <file> [--format json|text]"
    echo ""
    echo "Get LSP diagnostics (errors, warnings) for a file."
    echo ""
    echo "Arguments:"
    echo "  file      Path to the file to check"
    echo "  --format  Output format: text (default) or json"
    echo ""
    echo "Examples:"
    echo "  lsp-check src/main.rs"
    echo "  lsp-check app.tsx --format json"
    echo ""
    echo "Supported languages: Rust, Go, TypeScript, Python, and 20+ more"
    exit 1
fi

if [ ! -f "$FILE" ]; then
    echo "Error: File not found: $FILE"
    exit 1
fi

# Check if opencode is available
if ! command -v opencode &> /dev/null; then
    echo "Error: opencode not found in PATH"
    exit 1
fi

echo "Checking: $FILE"
echo "---"

# Run LSP diagnostics
opencode debug lsp diagnostics "$FILE"

echo "---"
echo "Skill: lsp-check"
```

#### 1.3 Create README files

**File**: `descartes/.descartes/skills/README.md`

```markdown
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
```

### Success Criteria:

#### Automated Verification:
- [ ] Skills are executable: `ls -la .descartes/skills/`
- [ ] opencode-delegate shows help: `.descartes/skills/opencode-delegate --help`
- [ ] lsp-check shows help: `.descartes/skills/lsp-check --help`
- [ ] Doctor detects skills: `cargo run --bin descartes -- doctor | grep "skills available"`

#### Manual Verification:
- [ ] opencode-delegate runs a simple prompt successfully
- [ ] lsp-check returns diagnostics for a Rust file

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation before proceeding to Phase 2.

---

## Phase 2: Server Management Skill

### Overview
Create a skill to manage the OpenCode server lifecycle, enabling persistent connections that avoid cold starts.

### Changes Required:

#### 2.1 Create opencode-server Skill

**File**: `descartes/.descartes/skills/opencode-server`

```bash
#!/bin/bash
# opencode-server - Manage OpenCode server lifecycle
#
# Usage: opencode-server <command> [options]
#
# Commands:
#   start   Start the OpenCode server
#   stop    Stop the OpenCode server
#   status  Check if server is running
#   url     Print the server URL

set -e

COMMAND="${1:-status}"
PORT="${OPENCODE_PORT:-4096}"
HOSTNAME="${OPENCODE_HOST:-127.0.0.1}"
PIDFILE="/tmp/opencode-server.pid"
LOGFILE="/tmp/opencode-server.log"

case "$COMMAND" in
    start)
        if [ -f "$PIDFILE" ] && kill -0 "$(cat $PIDFILE)" 2>/dev/null; then
            echo "OpenCode server already running (PID: $(cat $PIDFILE))"
            echo "URL: http://$HOSTNAME:$PORT"
            exit 0
        fi

        echo "Starting OpenCode server on $HOSTNAME:$PORT..."
        nohup opencode serve --port "$PORT" --hostname "$HOSTNAME" > "$LOGFILE" 2>&1 &
        echo $! > "$PIDFILE"

        # Wait for server to be ready
        for i in {1..10}; do
            if curl -s "http://$HOSTNAME:$PORT/health" > /dev/null 2>&1; then
                echo "Server started successfully"
                echo "URL: http://$HOSTNAME:$PORT"
                echo "PID: $(cat $PIDFILE)"
                exit 0
            fi
            sleep 0.5
        done

        echo "Warning: Server started but health check failed"
        echo "Check logs: $LOGFILE"
        ;;

    stop)
        if [ -f "$PIDFILE" ]; then
            PID=$(cat "$PIDFILE")
            if kill -0 "$PID" 2>/dev/null; then
                echo "Stopping OpenCode server (PID: $PID)..."
                kill "$PID"
                rm -f "$PIDFILE"
                echo "Server stopped"
            else
                echo "Server not running (stale PID file)"
                rm -f "$PIDFILE"
            fi
        else
            echo "Server not running (no PID file)"
        fi
        ;;

    status)
        if [ -f "$PIDFILE" ] && kill -0 "$(cat $PIDFILE)" 2>/dev/null; then
            echo "OpenCode server is running"
            echo "URL: http://$HOSTNAME:$PORT"
            echo "PID: $(cat $PIDFILE)"

            # Check health
            if curl -s "http://$HOSTNAME:$PORT/health" > /dev/null 2>&1; then
                echo "Health: OK"
            else
                echo "Health: UNHEALTHY"
            fi
        else
            echo "OpenCode server is not running"
            exit 1
        fi
        ;;

    url)
        if [ -f "$PIDFILE" ] && kill -0 "$(cat $PIDFILE)" 2>/dev/null; then
            echo "http://$HOSTNAME:$PORT"
        else
            echo "Error: Server not running"
            exit 1
        fi
        ;;

    logs)
        if [ -f "$LOGFILE" ]; then
            tail -50 "$LOGFILE"
        else
            echo "No logs found"
        fi
        ;;

    *)
        echo "Usage: opencode-server <command>"
        echo ""
        echo "Commands:"
        echo "  start   Start the OpenCode server"
        echo "  stop    Stop the OpenCode server"
        echo "  status  Check if server is running"
        echo "  url     Print the server URL"
        echo "  logs    Show recent server logs"
        echo ""
        echo "Environment variables:"
        echo "  OPENCODE_PORT  Server port (default: 4096)"
        echo "  OPENCODE_HOST  Server hostname (default: 127.0.0.1)"
        exit 1
        ;;
esac
```

#### 2.2 Update opencode-delegate to Auto-Attach

**File**: `descartes/.descartes/skills/opencode-delegate` (update)

Add auto-attach logic at the beginning of the execution section:

```bash
# Auto-attach to running server if no explicit --attach
if [ -z "$ATTACH_URL" ]; then
    # Check if server is running
    PIDFILE="/tmp/opencode-server.pid"
    if [ -f "$PIDFILE" ] && kill -0 "$(cat $PIDFILE)" 2>/dev/null; then
        ATTACH_URL="http://${OPENCODE_HOST:-127.0.0.1}:${OPENCODE_PORT:-4096}"
        echo "Auto-attaching to: $ATTACH_URL" >&2
    fi
fi
```

### Success Criteria:

#### Automated Verification:
- [ ] Server starts: `.descartes/skills/opencode-server start`
- [ ] Status shows running: `.descartes/skills/opencode-server status`
- [ ] URL is returned: `.descartes/skills/opencode-server url`
- [ ] Server stops: `.descartes/skills/opencode-server stop`
- [ ] Status shows stopped: `.descartes/skills/opencode-server status` (exit code 1)

#### Manual Verification:
- [ ] opencode-delegate auto-attaches when server is running
- [ ] Response time is faster with attach vs. cold start

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation before proceeding to Phase 3.

---

## Phase 3: LSP Fix Skill

### Overview
Create an advanced skill that combines LSP diagnostics with OpenCode to automatically fix issues.

### Changes Required:

#### 3.1 Create lsp-fix Skill

**File**: `descartes/.descartes/skills/lsp-fix`

```bash
#!/bin/bash
# lsp-fix - Get LSP diagnostics and ask OpenCode to fix them
#
# Usage: lsp-fix <file> [--dry-run]
#
# This skill:
# 1. Runs LSP diagnostics on the file
# 2. If errors found, sends them to OpenCode with the file content
# 3. OpenCode suggests or applies fixes

set -e

FILE="$1"
DRY_RUN=""

# Parse args
while [[ $# -gt 0 ]]; do
    case $1 in
        --dry-run)
            DRY_RUN="true"
            shift
            ;;
        -h|--help)
            echo "Usage: lsp-fix <file> [--dry-run]"
            echo ""
            echo "Get LSP diagnostics and ask OpenCode to fix them."
            echo ""
            echo "Options:"
            echo "  --dry-run   Show what would be fixed without making changes"
            echo "  -h, --help  Show this help"
            echo ""
            echo "Examples:"
            echo "  lsp-fix src/main.rs"
            echo "  lsp-fix app.tsx --dry-run"
            exit 0
            ;;
        *)
            if [ -z "$FILE" ]; then
                FILE="$1"
            fi
            shift
            ;;
    esac
done

if [ -z "$FILE" ]; then
    echo "Error: No file provided"
    echo "Usage: lsp-fix <file> [--dry-run]"
    exit 1
fi

if [ ! -f "$FILE" ]; then
    echo "Error: File not found: $FILE"
    exit 1
fi

# Check if opencode is available
if ! command -v opencode &> /dev/null; then
    echo "Error: opencode not found in PATH"
    exit 1
fi

echo "Checking $FILE for LSP diagnostics..."
echo "---"

# Get diagnostics
DIAGNOSTICS=$(opencode debug lsp diagnostics "$FILE" 2>&1) || true

if echo "$DIAGNOSTICS" | grep -q "No diagnostics"; then
    echo "No issues found!"
    exit 0
fi

echo "Found issues:"
echo "$DIAGNOSTICS"
echo "---"

if [ -n "$DRY_RUN" ]; then
    echo "[Dry run] Would ask OpenCode to fix these issues"
    exit 0
fi

# Build prompt with diagnostics
PROMPT="Fix the following LSP diagnostics in $FILE:

$DIAGNOSTICS

Please fix all the errors and warnings. Show me the corrected code."

# Check for running server
ATTACH_URL=""
PIDFILE="/tmp/opencode-server.pid"
if [ -f "$PIDFILE" ] && kill -0 "$(cat $PIDFILE)" 2>/dev/null; then
    ATTACH_URL="--attach http://${OPENCODE_HOST:-127.0.0.1}:${OPENCODE_PORT:-4096}"
fi

echo "Asking OpenCode to fix..."
echo "---"

opencode run $ATTACH_URL -q "$PROMPT"

echo "---"
echo "Skill: lsp-fix"
```

### Success Criteria:

#### Automated Verification:
- [ ] lsp-fix shows help: `.descartes/skills/lsp-fix --help`
- [ ] lsp-fix --dry-run works without making changes
- [ ] lsp-fix handles files with no issues gracefully

#### Manual Verification:
- [ ] lsp-fix correctly identifies errors in a file with issues
- [ ] lsp-fix produces reasonable fix suggestions from OpenCode

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation before proceeding to Phase 4.

---

## Phase 4: Documentation and Integration

### Overview
Update documentation and integrate OpenCode skills into the Descartes workflow.

### Changes Required:

#### 4.1 Update SKILLS.md

**File**: `descartes/docs/SKILLS.md`

Add section after "Example Skills":

```markdown
## OpenCode Integration Skills

Descartes includes built-in skills for integrating with [OpenCode](https://opencode.ai),
an open-source AI coding assistant with LSP support.

### Available OpenCode Skills

| Skill | Description |
|-------|-------------|
| `opencode-delegate` | Delegate prompts to OpenCode |
| `opencode-server` | Manage OpenCode server lifecycle |
| `lsp-check` | Get LSP diagnostics for a file |
| `lsp-fix` | Fix LSP errors using OpenCode |

### Usage Patterns

#### Quick Query (Cold Start)
```bash
.descartes/skills/opencode-delegate "Explain this code"
```

#### Fast Queries (With Server)
```bash
# Start server once
.descartes/skills/opencode-server start

# Multiple fast queries
.descartes/skills/opencode-delegate "Query 1"
.descartes/skills/opencode-delegate "Query 2"

# Stop when done
.descartes/skills/opencode-server stop
```

#### LSP Workflow
```bash
# Check for errors
.descartes/skills/lsp-check src/main.rs

# Auto-fix errors
.descartes/skills/lsp-fix src/main.rs
```

### Environment Variables

| Variable | Default | Description |
|----------|---------|-------------|
| `OPENCODE_PORT` | 4096 | Server port |
| `OPENCODE_HOST` | 127.0.0.1 | Server hostname |
```

#### 4.2 Update Doctor Command to Show OpenCode Status

**File**: `descartes/cli/src/commands/doctor.rs`

Add OpenCode check after skills check (around line 184):

```rust
// Check OpenCode availability
fn check_opencode() -> (Status, String) {
    match std::process::Command::new("opencode")
        .arg("--version")
        .output()
    {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout);
            let version = version.trim();
            (Status::Ok, format!("installed ({})", version))
        }
        Ok(_) => (Status::Warning, "installed but returned error".to_string()),
        Err(_) => (Status::NotConfigured, "not installed".to_string()),
    }
}
```

Add to the output section:

```rust
print_check("OpenCode", check_opencode());
```

### Success Criteria:

#### Automated Verification:
- [ ] SKILLS.md has OpenCode documentation: `grep -q "OpenCode Integration" descartes/docs/SKILLS.md`
- [ ] Doctor shows OpenCode status: `cargo run --bin descartes -- doctor | grep -i opencode`
- [ ] Build passes: `cargo build -p descartes-cli`

#### Manual Verification:
- [ ] Documentation is clear and accurate
- [ ] Doctor output includes OpenCode status line

**Implementation Note**: After completing this phase, all OpenCode skills integration is complete.

---

## Testing Strategy

### Unit Tests:
- Each skill's `--help` flag works
- Skills exit with proper codes (0 for success, 1 for errors)
- Skills handle missing dependencies gracefully

### Integration Tests:
- Start server, run delegate, stop server workflow
- LSP check on multiple file types
- LSP fix with real errors

### Manual Testing Steps:
1. Run `descartes doctor` and verify skills and OpenCode are detected
2. Start OpenCode server with `opencode-server start`
3. Run several `opencode-delegate` queries and verify speed improvement
4. Create a Rust file with errors, run `lsp-check`, then `lsp-fix`
5. Stop server with `opencode-server stop`

## Performance Considerations

- **Cold start**: `opencode run` takes 2-5 seconds for MCP initialization
- **Attached mode**: `opencode run --attach` responds in <1 second
- **Server memory**: OpenCode server uses ~100-200MB RAM
- **Recommendation**: Start server at beginning of session, stop at end

## Migration Notes

No migration needed - these are new skills that don't affect existing functionality.

## References

- Research document: `thoughts/shared/research/2025-12-25-opencode-server-lsp-integration.md`
- OpenCode TUI PRD: `.scud/docs/opencode_tui.md`
- Skills documentation: `descartes/docs/SKILLS.md`
- Example skill: `descartes/examples/skills/web-search/`
