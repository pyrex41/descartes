# HumanLayer / CodeLayer Project Structure Review

## 1. Project Overview
The project, currently transitioning from "HumanLayer SDK" to **"CodeLayer"**, is an open-source IDE designed to orchestrate AI coding agents (specifically built on Claude Code). Its goal is to enable AI agents to solve complex problems in large codebases by providing a "superhuman" interface and workflows. The legacy HumanLayer SDK, which focused on human-in-the-loop function calling (e.g., `@require_approval`), has been superseded.

The system appears to support "MultiClaude" (parallel sessions) and "Advanced Context Engineering".

## 2. Repository Architecture
The project is a **monorepo** managed with **Turbo** and uses **Bun** as the package manager and runtime.

### Key Configuration
- **`package.json`**: Defines the workspaces (`apps/*`, `packages/*`) and scripts for building, linting, and developing using Turbo.
- **`turbo.json`**: Configures the build pipeline, including tasks for `build`, `dev`, `lint`, and database operations.
- **`bun.lockb`**: Indicates the use of Bun for dependency management.

## 3. Core Components

### 3.1. Backend: `apps/daemon`
The `daemon` acts as the backend server for the application.
- **Runtime**: Bun.
- **Framework**: Uses **@orpc** (OpenRPC) for defining and handling API requests.
- **Entry Point**: `src/index.ts` sets up an `OpenAPIHandler` with CORS and error handling plugins.
- **Router**: `src/router/index.ts` (and `server.ts`, `sessions.ts`) likely defines the API endpoints.
- **Features**:
    - **SSE KeepAlive**: Configured for Server-Sent Events, suggesting real-time capabilities.
    - **Swagger/OpenAPI**: Includes generation of OpenAPI specs (`swagger.ts`).

### 3.2. Frontend: `apps/react`
The `react` app provides the user interface, likely the IDE itself.
- **Stack**: React, Vite (implied or similar bundler via Bun), Tailwind CSS, Shadcn UI.
- **Data Sync**: Uses **@electric-sql/react** (`useShape`) for local-first data synchronization with the backend/database. This is a key architectural choice for a responsive, offline-capable IDE.
- **Editor**: Contains a `components/Editor.tsx` and `y-electric` integration, indicating a collaborative or real-time text editing feature (likely for "thoughts" or code).
- **Key Components**:
    - `App.tsx`: Main entry point, handles routing or main layout, and demonstrates data fetching using `useShape` for `thoughts_documents`.
    - `components/ui`: Reusable UI components (Button, Card, Input, etc.) following the Shadcn pattern.

### 3.3. Shared Packages

#### `packages/contracts`
- **Purpose**: Defines the API contracts shared between the client and server.
- **Exports**: Exports `daemon` contract, ensuring type safety and consistency in API interactions.
- **Tech**: TypeScript.

#### `packages/database`
- **ORM**: **Drizzle ORM**.
- **Database**: PostgreSQL.
- **Schema**:
    - `thoughts.ts`: Defines `thoughts_documents`, `thoughts_documents_operations`, and `ydoc_awareness`. This confirms the focus on document editing and collaboration (Yjs integration).
    - `scores.ts`: A simple table for scores.
- **Migrations**: Managed via Drizzle Kit (`drizzle.config.ts`).

## 4. Auxiliary Components

### 4.1. `claudecode-go`
A Go module that appears to be a client library.
- **Purpose**: Likely allows Go-based tools or agents to interact with the CodeLayer system or Claude Code.
- **Files**: `client.go`, `types.go`, `doc.go`.

### 4.2. `hack/`
A collection of utility scripts and hacks.
- **`linear/`**: Integration with Linear (issue tracking), including a CLI and image fetching tests.
- **`dex/`**: Shell scripts (e.g., `flow-tmux.sh`).
- **Icon Generation**: Scripts for generating icons (`generate_nightly_icons.py`, etc.).
- **Worktree Management**: Scripts for managing git worktrees (`create_worktree.sh`, `cleanup_worktree.sh`), supporting the "MultiClaude" workflow.

### 4.3. `.claude/`
Configuration for Claude agents.
- **Agents**: Defines specific agent personas like `codebase-analyzer`, `codebase-locator`, `codebase-pattern-finder`, etc., with specific tool access and prompt instructions.
- **Commands**: Markdown files defining slash commands (e.g., `create_plan.md`, `debug.md`, `linear.md`) that likely drive the agent's behavior within the IDE.

### 4.4. `docs/`
Documentation for the project.
- **Case Studies**: e.g., `healthcare-case-study.md`.
- **Images**: Assets for docs.
- **Mintlify**: `mint.json` suggests the docs are published using Mintlify.

## 5. Key Technologies & Patterns
- **Bun**: Used extensively for speed and unified tooling (runtime, package manager, test runner).
- **Monorepo**: Efficient management of multiple apps and packages.
- **Local-First Sync**: ElectricSQL is used to sync data (like "thoughts") between the DB and the React frontend, providing a snappy user experience.
- **RPC**: @orpc provides a type-safe RPC layer.
- **Agentic Workflow**: The `.claude` directory and "thoughts" schema suggest a system deeply integrated with AI agents that plan, research, and execute tasks, with the "thoughts" documents serving as a shared context or scratchpad.

## 6. Summary
The **CodeLayer** project is a modern, sophisticated tool for AI-assisted software development. It leverages a high-performance stack (Bun, Drizzle, ElectricSQL) to build a responsive IDE (`apps/react`) backed by a robust daemon (`apps/daemon`). The architecture emphasizes real-time collaboration (Yjs, ElectricSQL) and structured agent interactions (defined in `.claude`). The transition from the legacy "HumanLayer" SDK is evident, with the new focus being on a holistic "Outer Loop" agent orchestration platform.
