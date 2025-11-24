# HumanLayer Daemon (HLD) Detailed Summary

## 1. Overview

The **HumanLayer Daemon (HLD)** is the core backend service of the HumanLayer platform. It acts as a centralized coordinator that manages **Claude Code sessions**, handles **human-in-the-loop approvals**, and provides a **real-time event streaming** interface. It exposes both a REST API and a JSON-RPC interface, allowing clients (like the TUI or CLI) to interact with sessions and approvals.

The daemon is designed to run locally, listening on a Unix socket (default: `~/.humanlayer/daemon.sock`) and an HTTP port (default: `7777`). It integrates with the **Model Context Protocol (MCP)** to intercept and manage tool use requests from AI agents.

## 2. Core Architecture

The HLD architecture is modular, with a central `Daemon` struct coordinating several specialized managers and servers.

### 2.1. Daemon Coordinator (`daemon/`)
- **Role**: The entry point and lifecycle manager of the application.
- **Responsibilities**:
  - Loads and validates configuration (`config/`).
  - Initializes the SQLite database connection (`store/`).
  - Sets up the internal Event Bus (`bus/`).
  - Instantiates the Session Manager and Approval Manager.
  - Starts the HTTP server and Unix socket listener.
  - Handles graceful shutdown via signal handling.
- **Key File**: `daemon/daemon.go` defines the `Daemon` struct and the `New()` and `Run()` methods.

### 2.2. API Layer (`api/`)
- **Role**: Provides the external interface for clients.
- **Implementation**: Uses `oapi-codegen` for type-safe Go code generation from an OpenAPI specification (`api/openapi.yaml`).
- **Handlers**:
  - **Sessions**: Create, list, update, and interrupt sessions.
  - **Approvals**: List pending approvals, decide (approve/deny) on requests.
  - **Files**: File system operations (likely for the AI to read/write).
  - **SSE**: Server-Sent Events for real-time updates to clients.
  - **Settings**: Manage user settings.
  - **Agents**: Discover and list available AI agents.

### 2.3. MCP Server (`mcp/`)
- **Role**: Implements the Model Context Protocol to interface with AI models (specifically Claude).
- **Functionality**:
  - Exposes a `request_approval` tool to the AI.
  - Intercepts tool calls that require human permission.
  - Suspends execution until a decision is received via the internal Event Bus.
  - Supports an `auto-deny` mode for testing.
- **Key Logic**: The `handleRequestApproval` method creates an approval record and waits for a response, bridging the gap between the synchronous AI tool call and the asynchronous human decision.

### 2.4. Session Management (`session/`)
- **Role**: Manages the lifecycle of Claude Code sessions.
- **Responsibilities**:
  - Creating new sessions with unique Run IDs.
  - Tracking session status (running, paused, error).
  - Managing permissions (e.g., "dangerous skip" permissions).
  - Interacting with the `store` to persist session state.
  - Monitoring permission expiry via `PermissionMonitor`.

### 2.5. Approval System (`approval/`)
- **Role**: Centralizes the logic for human-in-the-loop interactions.
- **Responsibilities**:
  - Creating approval requests from tool calls.
  - Storing approval state (pending, approved, denied).
  - Processing decisions from the API.
  - Notifying the MCP server (via Event Bus) when a decision is made.
  - Supports "local" approval management.

### 2.6. Storage Layer (`store/`)
- **Role**: Persistent storage for the daemon.
- **Implementation**: SQLite database (`~/.humanlayer/daemon.db`).
- **Data Models**:
  - **Sessions**: Metadata about AI sessions.
  - **Approvals**: Records of tool calls requiring permission and their outcomes.
  - **Conversation Events**: Logs of the interaction history.
- **Key Features**:
  - Uses `mattn/go-sqlite3`.
  - Supports database isolation for testing (in-memory or temp files).

### 2.7. Event Bus (`bus/`)
- **Role**: Internal pub/sub system for decoupling components.
- **Usage**:
  - The `ApprovalManager` publishes events when decisions are made.
  - The `MCPServer` subscribes to these events to unblock the AI.
  - The `SSEHandler` subscribes to stream updates to the UI.

## 3. Detailed Component Breakdown

### 3.1. Agent Discovery (`api/handlers/agents.go`)
The daemon includes a sophisticated agent discovery mechanism:
- **Locations**: Scans `.claude/agents` in both the user's home directory (global) and the current working directory (local).
- **Precedence**: Local agents override global agents with the same name.
- **Format**: Agents are defined in Markdown files with YAML frontmatter.
- **Fields**: `name`, `description`, `tools`, `model`.
- **Validation**: Ensures agent names contain only lowercase letters and hyphens.

### 3.2. Configuration (`config/config.go`)
The daemon is highly configurable via environment variables and build-time flags:
- **Socket Path**: `HUMANLAYER_DAEMON_SOCKET` (default: `~/.humanlayer/daemon.sock`)
- **Database Path**: `HUMANLAYER_DATABASE_PATH` (default: `~/.humanlayer/daemon.db`)
- **HTTP Port**: `HUMANLAYER_DAEMON_HTTP_PORT` (default: `7777`)
- **API Key**: `HUMANLAYER_API_KEY`
- **Debug Mode**: `HUMANLAYER_DEBUG=true` enables verbose logging.

### 3.3. Testing Infrastructure (`e2e/`, `TESTING.md`)
The project emphasizes reliability with a comprehensive testing strategy:
- **E2E Tests**: Located in `e2e/`, written in TypeScript (`test-rest-api.ts`).
  - Tests all 16 REST API endpoints.
  - Validates SSE streams and approval workflows.
  - Runs against a dedicated daemon instance.
- **Integration Tests**: Go tests with `tags=integration`.
  - Require database isolation (using `:memory:` or temp files).
  - Cover daemon startup, session flows, and MCP integration.
- **Manual Testing**: Instructions for using `nc` (netcat) to send JSON-RPC commands directly to the socket.

## 4. Future Roadmap (`TODO.md`)

The project has a clear roadmap for future improvements:
- **Performance**:
  - **Bulk Conversation History**: To solve N+1 query issues in the TUI.
  - **Full-Text Search**: Using SQLite FTS for searching session content.
- **Features**:
  - **Real-time Status**: Better propagation of "blocked" status when waiting for approvals.
  - **Session Metrics**: Tracking token usage, costs, and tool call statistics.
  - **Export API**: Exporting conversations to JSON/CSV/Markdown.
  - **Bulk Operations**: Deleting or archiving multiple sessions at once.
- **Technical Debt**:
  - **Event Bus**: Improving error handling and event persistence.
  - **Error Standardization**: Consistent error responses across the API.

## 5. Directory Structure Summary

- `api/`: REST/RPC API definition and handlers.
- `approval/`: Logic for managing approval requests.
- `bus/`: Internal event bus implementation.
- `client/`: Go client for interacting with the daemon.
- `cmd/`: Entry points (specifically `hld/main.go`).
- `config/`: Configuration loading and validation.
- `daemon/`: Main application logic and wiring.
- `e2e/`: End-to-end tests (TypeScript).
- `internal/`: Private utilities (filescan, version, etc.).
- `mcp/`: Model Context Protocol server implementation.
- `rpc/`: JSON-RPC specific handlers.
- `sdk/`: Generated client SDKs (TypeScript).
- `session/`: Session lifecycle management.
- `store/`: Database access layer.
