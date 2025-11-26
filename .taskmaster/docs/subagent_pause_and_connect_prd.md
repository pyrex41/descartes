# PRD: Subagent Pause & Connect Infrastructure

## Overview
Build first-class support for pausing any running subagent, capturing its execution context, and exposing a “connectable” handle that external tools (TUIs, IDEs, humans) can attach to. This feature is the foundation for richer experiences like attaching Claude Code or OpenCode sessions to the exact agent that needs human guidance.

## Problem / Opportunity
- Today agents run headlessly; once spawned they stream events but we cannot safely interrupt, inspect, or re-route control without killing the process.
- High-leverage workflows—reviewing plans, debugging live code, escalating to humans—require freezing the agent deterministically, surfacing its state, and letting external tools temporarily take over.
- Without pause/connect the promised “attach arbitrary TUIs to arbitrary subagents” cannot exist, blocking human-in-the-loop velocity and trust.

## Goals
1. Allow any LocalProcessRunner-backed agent (Claude Code, OpenCode, custom) to receive a `pause` command that reaches the underlying process and transitions the agent to a Paused state within the state machine.
2. Emit a stable “connect token” + metadata over RPC so clients (CLI, GUI, plugins) know how to attach.
3. Resume the agent once the external attachment is released, ensuring buffered stdout/stderr and registry state stay consistent.
4. Work across multiple concurrent agents without violating `max_concurrent_agents` or leaking OS handles.

## Non-goals
- Deep TUI integrations (handled in tool-specific PRDs).
- Time-travel debugging or deterministic replay.
- Support for remote runners that do not implement the pause signal.

## Personas & Use Cases
- **AI Orchestrator (primary)**: Needs to stop a runaway subagent, attach a specialized model, then resume.
- **Human Developer**: Pauses an agent to inspect intermediate files via Claude Code, then gives it back.
- **Ops Engineer**: Suspends long-running tasks during deployments or secrets rotation.

## User Stories
1. *As an orchestrator*, I can issue `descartes agents pause <id>` and see confirmation the agent is paused with attach info.
2. *As a GUI user*, I can click “Pause & Attach” on a swarm node and receive a connect link for external tools.
3. *As a script*, I can call `pause_agent` via RPC, open a Unix socket to the process, then `resume_agent`.

## Functional Requirements
1. **Pause Command Path**
   - CLI: `descartes agents pause/resume`.
   - RPC: new methods `agent.pause`, `agent.resume`.
   - GUI: event broadcast + button haptics.
2. **Process Control**
   - SIGSTOP/SIGCONT on Unix, pseudo console suspend on Windows (document limitations).
   - Timeout + escalation path (kill after N seconds if pause fails).
3. **State Machine**
   - Add `Paused` state with legal transitions.
   - Persist pause metadata (timestamp, reason, requested_by).
4. **Attachment Metadata**
   - Provide `connect_url`, `stdin/stdout socket path`, `auth_token`.
   - TTL + revocation support.
5. **Buffer Management**
   - Flush stdout/stderr to history before pause completes.
   - Resume streaming seamlessly.
6. **Concurrency Controls**
   - Paused agents still count toward concurrency until explicitly “detached”.
   - Optional config to free concurrency slots when paused > threshold.

## UX Flow (Happy Path)
1. User runs `descartes agents pause agent-123`.
2. CLI hits daemon RPC → runner issues pause signal → state becomes Pausing.
3. Runner confirms process stopped, creates attach socket + token, returns metadata.
4. CLI prints “Paused. Attach via `unix://...` by 2025-02-01T12:00Z”.
5. When user is done, they call resume (or tool releases token) → runner sends SIGCONT → state returns to Running.

## Technical Considerations
- **Runner updates**: LocalProcessRunner must track paused handles, intercept `wait` semantics, and guard against deadlocks when stdout pipes back up.
- **State Store**: Persist attach metadata so GUI/CLI can recover after daemon restart.
- **Auth**: Attach tokens hashed + stored; AuthManager enforces caller permissions.
- **Observability**: Emit structured events (`agent.pause.requested`, `agent.pause.completed`, `agent.attach.created`).
- **Compatibility**: no-op pause for backends lacking native signal support; surface capability flag.

## Telemetry & Success Metrics
- % of pause requests that succeed <2s.
- # of successful attachments per week.
- Mean time paused, to detect agents left frozen.
- Error taxonomy (unsupported, timeout, auth).

## Dependencies
- Agent state machine changes (core crate).
- RPC schema & client updates.
- CLI + GUI controls.
- Documentation + operator playbooks.

## Risks & Mitigations
- **Process Hung**: Pausing may deadlock if child expects input → mitigation: send pre-pause notification + configurable grace period.
- **Security**: Attach tokens could leak → mitigation: signed tokens, short TTL, ability to revoke.
- **Resource Leaks**: Extra sockets per attachment → mitigation: cleanup on resume/timeouts.

## Milestones
1. **Milestone A** – Core pause/resume plumbing & tests.
2. **Milestone B** – Metadata + RPC/CLI integration.
3. **Milestone C** – GUI + telemetry + docs.
4. **Milestone D** – GA (feature flag removed).
