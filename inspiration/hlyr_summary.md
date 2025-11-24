# HumanLayer CLI (`hlyr`) Summary

This document provides a detailed summary of the `hlyr` subdirectory within the HumanLayer project. `hlyr` is the Command Line Interface (CLI) tool for HumanLayer, providing direct human contact from the terminal, MCP server functionality, and integration with the Claude Code SDK.

## Directory Structure

```
hlyr/
├── hack/                   # Utility scripts for development/testing
│   └── test-local-approvals.ts # Script to test local approval workflows
├── src/                    # Source code
│   ├── commands/           # CLI command implementations
│   │   ├── claude/         # Claude Code configuration commands
│   │   │   └── init.ts     # `claude init` command
│   │   ├── thoughts/       # Thoughts management commands
│   │   │   ├── config.ts   # `thoughts config` command
│   │   │   ├── init.ts     # `thoughts init` command
│   │   │   ├── status.ts   # `thoughts status` command
│   │   │   ├── sync.ts     # `thoughts sync` command
│   │   │   └── uninit.ts   # `thoughts uninit` command
│   │   ├── claude.ts       # `claude` command entry point
│   │   ├── configShow.ts   # `config show` command
│   │   ├── joinWaitlist.ts # `join-waitlist` command
│   │   ├── launch.ts       # `launch` command
│   │   └── thoughts.ts     # `thoughts` command entry point
│   ├── utils/              # Utility functions
│   │   └── invocation.ts   # CLI invocation handling
│   ├── config.ts           # Configuration loading and resolution
│   ├── daemonClient.ts     # Client for communicating with the Daemon
│   ├── index.ts            # Main CLI entry point
│   ├── mcp.ts              # MCP server implementation
│   ├── mcpLogger.ts        # Logger for MCP server
│   └── thoughtsConfig.ts   # Configuration logic for thoughts system
├── tests/                  # End-to-end tests
│   ├── claudeInit.e2e.test.ts
│   └── configShow.e2e.test.ts
├── .eslintrc.json          # ESLint configuration
├── .gitignore              # Git ignore rules
├── .npmignore              # NPM ignore rules
├── CHANGELOG.md            # Version history
├── Makefile                # Build automation
├── package.json            # Node.js dependencies and scripts
├── prettier.config.cjs     # Prettier configuration
├── README.md               # General documentation
├── THOUGHTS.md             # Documentation for the Thoughts system
├── test_local_approvals.md # Documentation for testing local approvals
├── tsconfig.json           # TypeScript configuration
├── tsup.config.ts          # tsup build configuration
└── vitest.config.ts        # Vitest configuration
```

## Core Components

### 1. CLI Entry Point (`src/index.ts`)
The main entry point uses `commander` to define and parse CLI commands. It handles:
-   **Command Routing**: Dispatches commands like `launch`, `mcp`, `config`, `thoughts`, `claude`, etc.
-   **Invocation Handling**: Determines if the tool is run as `humanlayer`, `codelayer`, or `codelayer-nightly`.
-   **App Launching**: Can launch the desktop app if invoked without arguments (via `src/utils/invocation.ts`).

### 2. Daemon Client (`src/daemonClient.ts`)
Handles communication with the HumanLayer Daemon (`hld`) via a Unix socket (`~/.humanlayer/daemon.sock`).
-   **Protocol**: Uses JSON-RPC 2.0.
-   **Features**:
    -   `launchSession`: Starts a new Claude Code session.
    -   `subscribe`: Subscribes to real-time events (approvals, status changes).
    -   `createApproval`, `getApproval`, `sendDecision`: Manages the approval workflow.
    -   `health`: Checks daemon status.
-   **Resilience**: Includes retry logic for connection establishment.

### 3. MCP Server (`src/mcp.ts`)
Implements a Model Context Protocol (MCP) server (`humanlayer-claude-local-approvals`).
-   **Purpose**: Provides the `request_permission` tool to Claude Code.
-   **Workflow**:
    1.  Receives a tool call from Claude.
    2.  Connects to the daemon to create an approval request.
    3.  Polls the daemon for the approval status (approved/denied).
    4.  Returns the result to Claude.
-   **Logging**: Uses `src/mcpLogger.ts` to log MCP activities to `~/.humanlayer/logs/`.

### 4. Thoughts System (`src/commands/thoughts/`)
A system for managing developer notes separately from the code repository.
-   **Concept**: Keeps notes in a central `~/thoughts` repo but links them into projects via symlinks.
-   **Commands**:
    -   `init`: Sets up the thoughts structure (`thoughts/` directory with symlinks to user/shared/global folders) and installs git hooks.
    -   `sync`: Manually syncs thoughts to the central repo and updates the `searchable/` index (hard links for AI searchability).
    -   `status`: Shows the status of the thoughts repo (git status, uncommitted changes).
    -   `config`: Manages thoughts configuration.
    -   `uninit`: Removes the thoughts setup.
-   **Git Hooks**: Installs pre-commit (prevents committing `thoughts/`) and post-commit (auto-syncs thoughts) hooks.

### 5. Claude Code Configuration (`src/commands/claude/`)
-   **`init`**: An interactive wizard (using `@clack/prompts`) to initialize Claude Code configuration in a project.
    -   Copies commands, agents, and settings from the package to `.claude/`.
    -   Configures model (Opus, Sonnet, Haiku) and thinking settings.
    -   Updates `.gitignore`.

### 6. Configuration Management (`src/config.ts`)
-   **Resolution**: Resolves configuration from multiple sources in order of precedence: Flags > Environment Variables > Config File > Defaults.
-   **Schema**: Defines configuration for `www_base_url`, `daemon_socket`, and `run_id`.
-   **File Support**: Reads from `humanlayer.json` or `~/.config/humanlayer/humanlayer.json`.

## Development & Testing

-   **Build System**: Uses `tsup` to bundle the TypeScript code into a single ESM file (`dist/index.js`).
-   **Testing**:
    -   **Unit/E2E**: Uses `vitest` for testing (e.g., `tests/claudeInit.e2e.test.ts`).
    -   **Local Approvals**: `hack/test-local-approvals.ts` allows testing the full approval flow without the HumanLayer API, using the local daemon.
-   **Makefile**: Provides shortcuts for `build`, `test`, `lint`, `format`, etc.

## Key Features

-   **Local-First**: Heavily relies on the local daemon for state and operations, enabling offline-capable workflows (where applicable).
-   **Security**: The Thoughts system is designed to prevent accidental leakage of sensitive notes into code repositories.
-   **Integration**: Deep integration with Claude Code via MCP and configuration management.
-   **UX**: Focus on interactive, user-friendly CLI experiences (wizard-style inits, colorful output).
