# PRD: OpenCode TUI Attachment for Subagents

## Overview
Provide a parallel attachment flow for OpenCode (or other headless TUIs) so operators can pause any Descartes subagent and hand control to an OpenCode terminal UI instance. This supports teams who prefer open-source tooling, run on Linux servers, or need customizable keybindings/macros unavailable in Claude Code.

## Problem Statement
- Current orchestration lacks a vendor-neutral way to drop humans into an agent workspace.
- Many teams already standardized on OpenCode/Cline-style TUIs; forcing Claude Code blocks adoption.
- Without attachment, hybrid workflows (AI drafts, human refines, AI resumes) remain manual and error-prone.

## Goals
1. Launch OpenCode in “attached” mode against a paused agent workspace, with bidirectional streaming over Descartes’ pause/connect sockets.
2. Support both local and remote (SSH) terminals, documenting requirements for PTY forwarding.
3. Keep security + audit parity with Claude attachment, including tokens, telemetry, and resumability.

## User Stories
1. *As a backend engineer*, I pause a Rust subagent, attach via `opencode attach agent-42`, run failing tests interactively, then resume the agent.
2. *As a CICD operator*, I auto-pause agents touching infrastructure repos and require an OpenCode attachment with an approval checklist before continuing.
3. *As a remote team*, we run Descartes on a server; teammates SSH in, run OpenCode with `DESCARTES_ATTACH_TOKEN`, collaborate, then detach.

## Functional Requirements
1. **Entry Points**
   - CLI: `descartes agents attach opencode <agent-id> [--ssh user@host]`.
   - API: `agent.attach.opencode` RPC returning token + connection spec.
2. **Process Launch**
   - Local: spawn `opencode` binary with env pointing to attach sockets and repo root.
   - Remote: optionally run helper script that SSHs, exports env, and launches OpenCode there.
3. **Session Metadata**
   - Provide agent summary, pending tasks, and attach instructions in the OpenCode side panel.
   - Display live “Detaching returns control to Descartes” banner.
4. **Command Hooks**
   - `/descartes log "<note>"` to append human notes to agent event log.
   - `/descartes resume` to finish session without leaving TUI.
5. **Resilience**
   - If OpenCode exits unexpectedly, offer retry without losing pause state.
   - Heartbeat ping from OpenCode to daemon; auto-cleanup if heartbeat lost.
6. **Observability**
   - Metrics: attachment attempts, success, duration, resume outcomes separated by tool.
   - Structured events for auditing.

## Non-goals
- Replacing OpenCode’s UI; we simply launch/instrument it.
- Multi-user collaborative attachments (future).
- Supporting every possible TUI; we target OpenCode CLI and expose an extension spec for others.

## UX Outline
1. Operator runs attach command → CLI verifies prerequisites → issues pause via core feature.
2. Once paused, CLI either launches local OpenCode or prints SSH command for remote.
3. OpenCode shows attach banner + context file; user works as normal.
4. When exiting, OpenCode helper calls Descartes resume endpoint; CLI reports success and shows diff summary link.

## Technical Considerations
- **PTY Bridging**: Need pseudo-terminal bridging so interactive shells inside OpenCode still behave when outer process is paused.
- **File Sync**: Ensure agent workspace is accessible (NFS, git worktree). Document limitations when workspace lives on ephemeral tmpfs.
- **Token Handling**: Provide `DESCARTES_ATTACH_TOKEN` and `DESCARTES_AGENT_SOCKET` env; never print tokens in logs.
- **Extensibility**: Define `attach_manifest.json` so other TUIs can plug in with minimal code.
- **Feature Flag**: `feature.attach.opencode` default off until beta feedback.

## Dependencies
- Subagent Pause & Connect infrastructure.
- CLI + daemon support for tool-specific attach flows.
- Optional helper scripts packaged with CLI for SSH workflows.

## Telemetry / Success Metrics
- Attachment success rate, mean connect time.
- % of attachments that lead to successful resume vs abandon.
- User satisfaction pulse after detach (CLI prompt).

## Risks & Mitigations
- **OpenCode version drift** → add compat matrix + runtime check.
- **Remote file perms** → preflight check ensures user has write access, else abort with guidance.
- **Security** → enforce same auth controls as Claude attach; tokens bound to tool type.

## Milestones
1. Prototype (local attach, manual resume).
2. Beta (CLI polish, remote attach helper, telemetry).
3. GA (docs, support playbooks, feature flag off).
