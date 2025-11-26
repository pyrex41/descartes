# PRD: Claude Code TUI Attachment for Subagents

## Overview
Deliver an experience where operators can pause any Claude-based subagent inside Descartes and seamlessly attach it to a live Claude Code TUI session for collaborative debugging, planning, or code edits. The Claude session should inherit the agent’s working directory, environment, and context, then hand control back to the orchestrator on release.

## Problem Statement
- Claude Code shines for iterative coding, but today Descartes runs Claude agents headlessly with no way to “drop in” mid-run.
- Human reviewers want to jump into the exact agent workspace (files, git worktree, env vars) without rehydrating context manually.
- Lack of attachment prevents critical workflows: human approval of risky migrations, multi-model hand-offs, live pair-programming with the agent.

## Objectives
1. One-command attach: `descartes agents attach claude-code <agent-id>` pauses the agent, launches Claude Code pointed at the agent workspace, and bridges stdio over secure sockets.
2. Preserve agent context (system prompt, scratchpad, env, repo) so Claude Code immediately understands what the agent was doing.
3. Allow bi-directional control: Claude Code edits files / runs commands, while Descartes records the session to agent history for auditability.
4. Support detach/resume so the automated agent can keep running once the human is done.

## Key Scenarios
1. **Plan Review** – Agent reaches “await review” state, operator attaches Claude Code to inspect TODO.md and update plans before resuming.
2. **Live Debug** – Agent hits compile errors; human attaches, runs the same tests inside Claude Code terminal, fixes code, then returns control.
3. **Escalation** – Automated guardrail triggers (e.g., touching prod), auto-pauses agent and prompts on-call to attach via Claude Code.

## Functional Requirements
1. **Launch Flow**
   - CLI command + GUI button to request Claude attachment.
   - Detect local Claude CLI install; if absent, show actionable error.
2. **Session Bootstrap**
   - Generate temporary Claude project config inside agent workspace (e.g., `.claude/session.json`).
   - Provide custom slash command to “return control” (calls Descartes resume endpoint).
3. **Transport**
   - Reuse pause/connect sockets; provide env vars `DESCARTES_ATTACH_TOKEN`, `DESCARTES_AGENT_ID`.
   - Stream stdout/stderr from Claude Code back into Descartes event log (`agent.attach.stream`).
4. **Context Sync**
   - Prepend session summary: active task, DAG node, recent events.
   - Optionally seed Claude conversation with latest agent thoughts (pending privacy review).
5. **Lifecycle**
   - Timeout if attachment not established within N seconds (default 120).
   - Auto-resume (or ask) when Claude session exits.
   - If Claude Code crashes, allow retry without new pause.
6. **Audit & Observability**
   - Record who attached, when, commands executed (if user consents/logging enabled).
   - Emit metrics for dwell time, resume success.

## Non-Goals
- Running Claude Code in headless/server mode (desktop app assumed).
- Supporting remote Claude sessions over SSH (documented workaround only).
- Multi-user simultaneous attachments (future).

## UX Outline
1. Operator issues attach command → CLI pauses agent (via core feature) → prints instructions:
   ```
   Agent paused. Launching Claude Code...
   Run: CLAUDE_PROJECT_ROOT=/tmp/... claude /attach --token=XYZ
   ```
2. Claude Code opens in target directory, loads slash commands for Descartes (pause/resume, log upload).
3. When finished, user runs `/return-to-descartes` or closes app → daemon resumes agent and posts summary comment to agent history.

## Technical Design Considerations
- **Worktree Mounting**: Ensure agent workspace is on disk where Claude Code can read/write; for remote runners, leverage SSHFS or block feature with clear message.
- **Prompt Injection**: Provide sanitized context to Claude; avoid leaking orchestration secrets.
- **Event Recording**: Hook into agent event emitter so edits executed during Claude attachment appear in the same timeline (diff summaries, git status).
- **Security**: Attachment token limited to invoking `claude` binary with specific socket; tokens expire after single use.
- **Configurability**: Support per-install defaults (auto-launch vs manual), feature flag for beta rollout.

## Integrations & Dependencies
- Requires Subagent Pause & Connect Infrastructure.
- CLI + daemon RPC updates for `agents.attach.claude`.
- Optional GUI updates (button, status pill).
- Documentation: operator guide, troubleshooting (e.g., missing CLI, macOS sandbox prompts).

## Telemetry & Success Metrics
- Attachment success rate.
- Avg time from attach request to Claude ready screen.
- % sessions that resume successfully.
- User feedback (thumbs) after session completion.

## Risks / Mitigations
- **Claude CLI API changes** → version pin + health check before attachment.
- **Long pauses** → remind operator after threshold, auto-resume if idle > configurable limit.
- **Security** → restrict attach tokens to localhost, enforce TLS when remote.

## Rollout Plan
1. Private beta (internal team) behind feature flag.
2. Public beta for power users with telemetry + feedback form.
3. GA once stability + UX polished, docs published, support runbooks ready.
