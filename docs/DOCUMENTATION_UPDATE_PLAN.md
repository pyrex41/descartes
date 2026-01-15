# Documentation Update Plan

**Date**: 2025-01-15
**Based on**: Research document `thoughts/research/2025-01-15-documentation-vs-codebase-analysis.md`

## Overview

This plan addresses 4 major documentation inaccuracies discovered through codebase analysis:

1. **Harness descriptions** - OpenCode incorrectly described as "TUI with IPC"
2. **Authentication documentation** - Wrong guidance for claude-code and opencode harnesses
3. **Installation instructions** - GUI doesn't install CLI, needs clarification
4. **Execution modes** - No distinction between `ralph` and `loop` commands

## Phase 1: Harness Documentation Fixes

### File: `docs/harnesses.html`

#### Change 1.1: Fix OpenCode description
**Location**: Line 137-138

**Current**:
```html
<tr>
    <td><code>opencode</code></td>
    <td>TUI with IPC</td>
```

**Replace with**:
```html
<tr>
    <td><code>opencode</code></td>
    <td>Headless CLI</td>
```

#### Change 1.2: Fix detailed OpenCode card
**Location**: Lines 198-235

**Current** (line 199):
```html
<p>Connects to OpenCode's TUI via IPC. Best for interactive development with visual feedback.</p>
```

**Replace with**:
```html
<p>Runs OpenCode CLI in headless mode via subprocess. Uses Grok models for fast iteration.</p>
```

**Current** (lines 224-229):
```html
<h4>How It Works</h4>
<ol>
    <li>Starts OpenCode process with IPC channel</li>
    <li>Sends prompts via stdin</li>
    <li>Receives responses via stdout</li>
    <li>OpenCode provides TUI visualization</li>
</ol>
```

**Replace with**:
```html
<h4>How It Works</h4>
<ol>
    <li>Spawns <code>opencode run --format json</code> as subprocess</li>
    <li>Streams prompts via stdin</li>
    <li>Captures JSON responses from stdout</li>
    <li>Closes subprocess when task completes</li>
</ol>
```

#### Change 1.3: Update "Best For" callout
**Location**: Lines 231-234

**Current**:
```html
<div class="callout">
    <strong>Best For</strong>
    Interactive development where you want to see agent activity. Uses Grok models for fast iteration.
</div>
```

**Replace with**:
```html
<div class="callout">
    <strong>Best For</strong>
    Fast iteration with Grok models. Headless execution like claude-code, but with xAI's infrastructure.
</div>
```

---

## Phase 2: Authentication Documentation Fixes

### File: `docs/harnesses.html`

#### Change 2.1: Fix Claude Code auth section
**Location**: Lines 163-168

**Current**:
```html
<h4>Setup</h4>
<pre><code># Install Claude CLI
npm install -g @anthropic-ai/claude-cli

# Set API key
export ANTHROPIC_API_KEY=sk-ant-api03-...</code></pre>
```

**Replace with**:
```html
<h4>Setup</h4>
<pre><code># Install Claude CLI
npm install -g @anthropic-ai/claude-cli

# Authenticate (Descartes delegates to Claude CLI)
claude login</code></pre>
```

#### Change 2.2: Fix OpenCode auth section
**Location**: Lines 202-207

**Current**:
```html
<h4>Setup</h4>
<pre><code># Install OpenCode
cargo install opencode

# Set API key (xAI/Grok models)
export XAI_API_KEY=xai-...</code></pre>
```

**Replace with**:
```html
<h4>Setup</h4>
<pre><code># Install OpenCode
cargo install opencode

# Set API key for OpenCode CLI (Descartes delegates to OpenCode)
# For xAI/Grok models:
export XAI_API_KEY=xai-...
# Or for Anthropic models:
export ANTHROPIC_API_KEY=sk-ant-...</code></pre>
```

#### Change 2.3: Fix Environment Variables table
**Location**: Lines 362-371

**Current**:
```html
<table>
    <tr><th>Variable</th><th>Harness</th><th>Description</th></tr>
    <tr><td><code>ANTHROPIC_API_KEY</code></td><td>claude-code</td><td>Claude API authentication</td></tr>
    <tr><td><code>XAI_API_KEY</code></td><td>opencode</td><td>xAI/Grok API authentication</td></tr>
    <tr><td><code>OPENAI_API_KEY</code></td><td>codex</td><td>OpenAI API authentication</td></tr>
```

**Replace with**:
```html
<table>
    <tr><th>Variable</th><th>Used By</th><th>Description</th></tr>
    <tr><td><code>ANTHROPIC_API_KEY</code></td><td>BAML, opencode CLI</td><td>Required for BAML orchestration (pure loop mode). OpenCode CLI may also use this.</td></tr>
    <tr><td><code>XAI_API_KEY</code></td><td>opencode CLI</td><td>OpenCode CLI reads this for xAI/Grok models</td></tr>
    <tr><td><code>OPENAI_API_KEY</code></td><td>codex harness (direct)</td><td>Descartes reads this directly for Codex harness</td></tr>
```

#### Change 2.4: Add auth clarification callout
**Location**: After line 371, add new callout

**Add**:
```html
<div class="callout">
    <strong>Authentication Note</strong>
    The claude-code and opencode harnesses delegate authentication to their respective CLIs.
    Descartes does not read API keys for these harnesses - run <code>claude login</code> or
    set environment variables for the underlying CLI tools. Only the codex harness reads
    <code>OPENAI_API_KEY</code> directly.
</div>
```

---

## Phase 3: Installation Documentation Fixes

### File: `docs/getting-started.html`

#### Change 3.1: Fix installation section
**Location**: Lines 118-126

**Current**:
```html
<h2>Installation</h2>

<h3>From crates.io</h3>
<pre><code>cargo install descartes-cli</code></pre>

<h3>From Source</h3>
<pre><code>git clone https://github.com/anthropics/descartes
cd descartes/descartes
cargo build --release</code></pre>
```

**Replace with**:
```html
<h2>Installation</h2>

<h3>CLI Only</h3>
<pre><code>cargo install descartes-cli</code></pre>

<h3>CLI + GUI</h3>
<pre><code># Both commands required - GUI does not install CLI
cargo install descartes-cli descartes-gui</code></pre>

<div class="callout">
    <strong>Note on Packaging</strong>
    The GUI and CLI are separate crates. The GUI depends on the CLI library but
    does not install its binaries. This is intentional to keep the CLI lean -
    the Iced GUI framework adds ~100 transitive dependencies.
</div>

<h3>From Source</h3>
<pre><code>git clone https://github.com/anthropics/descartes
cd descartes/descartes
cargo build --release

# Optional: build GUI too
cd ../descartes-gui
cargo build --release</code></pre>
```

### File: `docs/gui-guide.html`

#### Change 3.2: Add CLI requirement note
**Location**: Lines 120-125, update installation section

**Current**:
```html
<h2>Installation</h2>

<pre><code># From crates.io
cargo install descartes-gui

# From source
cd descartes/descartes-gui
cargo build --release</code></pre>
```

**Replace with**:
```html
<h2>Installation</h2>

<pre><code># Install both CLI and GUI (GUI requires CLI library, but CLI binaries installed separately)
cargo install descartes-cli descartes-gui

# From source
cd descartes/descartes-gui
cargo build --release</code></pre>

<div class="callout warning">
    <strong>CLI Not Included</strong>
    Installing the GUI does not install the <code>descartes</code> CLI binary.
    Run both <code>cargo install</code> commands to get full functionality.
</div>
```

---

## Phase 4: Execution Modes Documentation

### File: `docs/cli-reference.html`

#### Change 4.1: Add execution modes overview section
**Location**: After the Global Options section (line 130), add new section

**Add**:
```html
<h2>Execution Modes</h2>

<p>Descartes has two distinct execution patterns, each optimized for different workflows:</p>

<table>
    <tr>
        <th>Aspect</th>
        <th><code>descartes ralph</code></th>
        <th><code>descartes loop</code></th>
    </tr>
    <tr>
        <td>Pattern</td>
        <td>Fresh context per task</td>
        <td>BAML-orchestrated iteration</td>
    </tr>
    <tr>
        <td>Review Agent</td>
        <td>No (intentional for determinism)</td>
        <td>Yes (conditional on config)</td>
    </tr>
    <tr>
        <td>Context Handoff</td>
        <td>Yes (at 60% window)</td>
        <td>No</td>
    </tr>
    <tr>
        <td>BAML Usage</td>
        <td>None</td>
        <td>Heavy (4 functions)</td>
    </tr>
    <tr>
        <td>Best For</td>
        <td>CI/CD, large task graphs</td>
        <td>Interactive development</td>
    </tr>
</table>

<div class="callout">
    <strong>Why Two Modes?</strong>
    The <code>ralph</code> command prioritizes determinism and parallelism for batch execution.
    The <code>loop</code> command uses BAML for intelligent decisions and includes an optional
    review agent for quality-critical work. Choose based on your workflow needs.
</div>
```

#### Change 4.2: Update ralph command description
**Location**: Lines 134-193 (ralph command section)

**Current** (line 136-137):
```html
<h3>descartes ralph</h3>
<p>Execute SCUD tasks using the Ralph Wiggum loop pattern. This is the primary command for AI-driven task execution.</p>
```

**Replace with**:
```html
<h3>descartes ralph</h3>
<p>Execute SCUD tasks with fresh context per task and wave-based parallelism. Best for batch execution and CI/CD.</p>

<div class="callout">
    <strong>Key Characteristics</strong>
    <ul style="margin-top: 8px;">
        <li><strong>Fresh context</strong>: Each task starts with clean agent session</li>
        <li><strong>Context handoff</strong>: Long tasks automatically hand off at 60% context window</li>
        <li><strong>No review agent</strong>: Uses backpressure validation instead (intentional for determinism)</li>
        <li><strong>Wave parallelism</strong>: Tasks in same wave run in configurable rounds</li>
    </ul>
</div>
```

#### Change 4.3: Update loop command description
**Location**: Lines 234-249 (loop command section)

**Current** (line 237-238):
```html
<h3>descartes loop</h3>
<p>Run the Ralph loop continuously.</p>
```

**Replace with**:
```html
<h3>descartes loop</h3>
<p>Run BAML-orchestrated iterations with optional review agent. Best for interactive development.</p>

<div class="callout">
    <strong>Key Characteristics</strong>
    <ul style="margin-top: 8px;">
        <li><strong>BAML decisions</strong>: Uses AI to decide next action (Continue/Replan/Complete/AskHuman)</li>
        <li><strong>5-phase pattern</strong>: Parallel search → Builder → Review → Validator → Commit</li>
        <li><strong>Review agent</strong>: Optional code review (enable with <code>always_review</code> config)</li>
        <li><strong>Dynamic agent selection</strong>: BAML chooses searcher/builder categories</li>
    </ul>
</div>
```

#### Change 4.4: Add review agent configuration
**Location**: After loop command table (line 248)

**Add**:
```html
<h4>Review Agent Configuration</h4>
<p>The review agent is conditional in loop mode:</p>
<pre><code># In descartes.toml
[ralph_loop]
always_review = true  # Enable review agent for all builds

# Or via task override (YAML frontmatter in task):
---
category: fast-builder
disable_review: true  # Skip review for this task
---</code></pre>
```

---

## Phase 5: Attach Mode Documentation

### File: `docs/gui-guide.html`

#### Change 5.1: Add attach mode explanation
**Location**: After Agents View section (line 191)

**Add**:
```html
<h3>Terminal Attach Mode</h3>

<p>When running agents in a terminal multiplexer (Zellij, Tmux, Kitty), you can focus on a running agent's pane:</p>

<ol>
    <li>Press <strong>1-9</strong> in the Ralph TUI to focus on that agent's pane</li>
    <li>Descartes detects your terminal type automatically</li>
    <li>Focus commands: <code>zellij action focus-pane</code>, <code>tmux select-pane</code>, <code>kitty @ focus-window</code></li>
</ol>

<div class="callout">
    <strong>Multiplexer Required</strong>
    Attach mode requires running Descartes in a terminal multiplexer. The GUI's Agents view
    provides Pause/Resume controls as an alternative for headless operation.
</div>
```

### File: `docs/harnesses.html`

#### Change 5.2: Clarify attach is via terminal multiplexer
**Location**: After "All harnesses run headless" note (add new section after Agent Categories table, ~line 309)

**Add**:
```html
<h2>Attach Mode</h2>

<p>All harnesses run headless, but you can attach to running agents via terminal multiplexers:</p>

<h3>Terminal Multiplexer Attach</h3>
<p>When running in Zellij, Tmux, or Kitty:</p>
<ul>
    <li>Press <strong>1-9</strong> in the Ralph TUI to focus on agent panes</li>
    <li>Descartes auto-detects your terminal type</li>
    <li>Works with: Zellij, Tmux, Kitty</li>
</ul>

<h3>GUI Attach</h3>
<p>The Descartes GUI provides Pause/Resume controls in the Agents view, allowing you to suspend and resume agent execution without terminal multiplexer.</p>
```

---

## Implementation Order

1. **Phase 1** (Harness fixes) - Most critical, prevents user confusion
2. **Phase 2** (Auth fixes) - Prevents setup failures
3. **Phase 3** (Installation fixes) - Helps new users
4. **Phase 4** (Execution modes) - Helps advanced users choose
5. **Phase 5** (Attach mode) - Documents existing feature

## Verification

After implementing all phases:

1. **Visual check**: Open each HTML file in browser, verify rendering
2. **Link check**: Verify all internal links work
3. **Code reference check**: Verify code examples match actual CLI help output
4. **Consistency check**: Verify terminology is consistent across all docs

## Success Criteria

- [x] OpenCode described as "Headless CLI" not "TUI with IPC"
- [x] claude-code setup shows `claude login` not `ANTHROPIC_API_KEY`
- [x] Installation docs show both `cargo install` commands
- [x] Execution modes comparison table exists
- [x] Review agent configuration documented
- [x] Attach mode via terminal multiplexer documented
