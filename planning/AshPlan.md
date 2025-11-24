# Descartes AI Orchestration Platform - Product Requirements Document
## Ash Framework + Phoenix LiveView Implementation

### Executive Summary

Descartes is a cloud-native AI agent orchestration platform that implements an "Architect → Plan → Swarm" workflow for automated software development. Built on Elixir's Ash Framework and Phoenix LiveView, it provides real-time orchestration of AI agents, intelligent task decomposition, and knowledge graph-powered context management for complex software projects.

---

## 1. Product Vision & Objectives

### Vision Statement
Enable developers to orchestrate swarms of specialized AI agents that collaboratively build software through intelligent planning, autonomous execution, and continuous learning.

### Core Objectives
- **Reduce Development Time**: 10x productivity gains through AI agent orchestration
- **Improve Code Quality**: Systematic architecture and planning before implementation
- **Enable Complex Projects**: Break down ambitious projects into manageable, parallel workstreams
- **Continuous Learning**: Build organizational knowledge graphs from every project

### Key Differentiators
- **Cloud-Native Architecture**: Long-running orchestration processes managed server-side
- **Real-Time Collaboration**: Watch agents work in real-time via Phoenix LiveView
- **Knowledge Persistence**: Every decision and artifact feeds into a searchable knowledge graph
- **Declarative Orchestration**: Define workflows as data, not code

---

## 2. System Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Phoenix LiveView UI                      │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │Dashboard │  │ Workflows │  │Knowledge │  │  Agents  │   │
│  │  View    │  │  Editor   │  │  Graph   │  │ Monitor  │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
└───────────────────────┬─────────────────────────────────────┘
                        │ WebSocket (Phoenix Channels)
┌───────────────────────┴─────────────────────────────────────┐
│                    Ash Application Layer                     │
│  ┌──────────────────────────────────────────────────────┐   │
│  │              Ash Resources & Actions                  │   │
│  │  • Projects  • Workflows  • Agents  • Knowledge      │   │
│  │  • Tasks     • Plans      • Tools   • Artifacts      │   │
│  └──────────────────────────────────────────────────────┘   │
│  ┌──────────────────────────────────────────────────────┐   │
│  │           Orchestration Engine (GenServers)          │   │
│  │  • Agent Supervisors  • Workflow Executors           │   │
│  │  • Task Schedulers    • Event Processors             │   │
│  └──────────────────────────────────────────────────────┘   │
└───────────────────────┬─────────────────────────────────────┘
                        │
┌───────────────────────┴─────────────────────────────────────┐
│                    External Services                         │
│  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌──────────┐   │
│  │   LLM    │  │  GitHub  │  │    S3    │  │  Vector  │   │
│  │   APIs   │  │   API    │  │  Storage │  │    DB    │   │
│  └──────────┘  └──────────┘  └──────────┘  └──────────┘   │
└──────────────────────────────────────────────────────────────┘
```

### Technology Stack

- **Backend Framework**: Ash 3.x + Phoenix 1.7
- **Real-time UI**: Phoenix LiveView
- **Database**: PostgreSQL 15+ with pgvector extension
- **Knowledge Graph**: PostgreSQL with graph extensions + FTS
- **Background Jobs**: Oban
- **Caching**: Redis
- **File Storage**: S3-compatible (AWS/Minio)
- **Deployment**: Kubernetes / Fly.io
- **Monitoring**: Telemetry + Grafana

---

## 3. Core Features & User Stories

### 3.1 Project Management

**User Story**: As a developer, I want to create a project and define high-level goals so that AI agents can architect and build my software.

**Features**:
- Project creation with natural language descriptions
- Technology stack selection and constraints
- Budget limits (token usage, compute time)
- Success criteria definition

### 3.2 Architect Agent

**User Story**: As a developer, I want an AI architect to analyze my requirements and create a comprehensive technical plan.

**Features**:
- Requirements analysis and clarification
- Architecture document generation
- Technology recommendations
- Task decomposition into epics and stories
- Dependency mapping

### 3.3 Planning Engine

**User Story**: As a developer, I want the system to create an optimal execution plan that maximizes parallelization while respecting dependencies.

**Features**:
- DAG-based task scheduling
- Critical path analysis
- Resource allocation optimization
- Time and cost estimation
- Risk assessment

### 3.4 Agent Swarm Orchestration

**User Story**: As a developer, I want specialized agents to execute tasks in parallel while maintaining consistency.

**Features**:
- Agent spawning based on task requirements
- Inter-agent communication protocols
- Shared context management
- Conflict resolution
- Progress synchronization

### 3.5 Real-Time Monitoring

**User Story**: As a developer, I want to watch agents work in real-time and intervene when necessary.

**Features**:
- Live agent status dashboard
- Streaming logs and outputs
- Task progress visualization
- Token usage monitoring
- Manual intervention capabilities
- Breakpoint debugging

### 3.6 Knowledge Graph

**User Story**: As a team, we want to build a searchable knowledge base from all our AI-assisted development.

**Features**:
- Automatic knowledge extraction from agent decisions
- Semantic search across all projects
- Pattern recognition and suggestions
- Code snippet library
- Architecture pattern catalog
- Problem-solution mapping

---

## 4. Data Models (Ash Resources)

### 4.1 Core Resources

```elixir
# Project Resource
defmodule Descartes.Projects.Project do
  use Ash.Resource,
    data_layer: AshPostgres.DataLayer,
    authorizers: [Ash.Policy.Authorizer]

  attributes do
    uuid_primary_key :id
    attribute :name, :string, allow_nil?: false
    attribute :description, :text
    attribute :status, :atom, 
      constraints: [one_of: [:draft, :planning, :executing, :completed, :failed]],
      default: :draft
    attribute :config, :map, default: %{}
    attribute :metrics, :map, default: %{}
    timestamps()
  end

  relationships do
    belongs_to :organization, Descartes.Accounts.Organization
    belongs_to :owner, Descartes.Accounts.User
    has_many :workflows, Descartes.Orchestration.Workflow
    has_many :artifacts, Descartes.Storage.Artifact
    has_many :knowledge_nodes, Descartes.Knowledge.Node
  end

  actions do
    defaults [:read, :destroy]
    
    create :create do
      primary? true
      argument :organization_id, :uuid, allow_nil?: false
      change relate_to(:organization, :organization_id)
    end
    
    update :start_planning do
      validate attribute_equals(:status, :draft)
      change set_attribute(:status, :planning)
      change Descartes.Changes.SpawnArchitect
    end
    
    update :start_execution do
      validate attribute_equals(:status, :planning)
      change set_attribute(:status, :executing)
      change Descartes.Changes.SpawnSwarm
    end
  end

  policies do
    policy action_type(:read) do
      authorize_if relates_to_actor_via(:organization)
    end
    
    policy action_type([:create, :update, :destroy]) do
      authorize_if relates_to_actor_via(:owner)
    end
  end
end

# Workflow Resource
defmodule Descartes.Orchestration.Workflow do
  use Ash.Resource,
    data_layer: AshPostgres.DataLayer,
    extensions: [AshStateMachine]

  attributes do
    uuid_primary_key :id
    attribute :type, :atom,
      constraints: [one_of: [:architect, :planner, :executor, :reviewer]]
    attribute :config, :map
    attribute :state_data, :map, default: %{}
    attribute :started_at, :utc_datetime_usec
    attribute :completed_at, :utc_datetime_usec
  end

  state_machine do
    initial_states [:pending]
    default_initial_state :pending
    
    transitions do
      transition :start, from: :pending, to: :running
      transition :pause, from: :running, to: :paused
      transition :resume, from: :paused, to: :running
      transition :complete, from: :running, to: :completed
      transition :fail, from: [:running, :paused], to: :failed
      transition :retry, from: :failed, to: :pending
    end
  end

  relationships do
    belongs_to :project, Descartes.Projects.Project
    has_many :agents, Descartes.Agents.Agent
    has_many :tasks, Descartes.Tasks.Task
  end
end

# Agent Resource
defmodule Descartes.Agents.Agent do
  use Ash.Resource,
    data_layer: AshPostgres.DataLayer

  attributes do
    uuid_primary_key :id
    attribute :name, :string, allow_nil?: false
    attribute :type, :atom,
      constraints: [one_of: [:architect, :developer, :tester, :reviewer, :specialist]]
    attribute :capabilities, {:array, :string}, default: []
    attribute :model, :string, default: "gpt-4-turbo"
    attribute :temperature, :float, default: 0.7
    attribute :status, :atom,
      constraints: [one_of: [:idle, :thinking, :executing, :waiting, :failed]],
      default: :idle
    attribute :memory, :map, default: %{}
    attribute :token_usage, :map, default: %{input: 0, output: 0}
    attribute :process_pid, :string
  end

  relationships do
    belongs_to :workflow, Descartes.Orchestration.Workflow
    has_many :assigned_tasks, Descartes.Tasks.Task
    has_many :messages, Descartes.Agents.Message
    has_many :tool_calls, Descartes.Tools.ToolCall
  end

  actions do
    create :spawn do
      argument :workflow_id, :uuid, allow_nil?: false
      argument :type, :atom, allow_nil?: false
      change Descartes.Changes.StartAgentProcess
    end
    
    update :assign_task do
      argument :task_id, :uuid, allow_nil?: false
      change set_attribute(:status, :thinking)
      change relate_to(:assigned_tasks, :task_id)
    end
    
    update :execute_tool do
      argument :tool, :atom, allow_nil?: false
      argument :parameters, :map
      change set_attribute(:status, :executing)
      change Descartes.Changes.ExecuteTool
    end
  end
end

# Task Resource  
defmodule Descartes.Tasks.Task do
  use Ash.Resource,
    data_layer: AshPostgres.DataLayer

  attributes do
    uuid_primary_key :id
    attribute :title, :string, allow_nil?: false
    attribute :description, :text
    attribute :type, :atom,
      constraints: [one_of: [:epic, :story, :task, :subtask]]
    attribute :status, :atom,
      constraints: [one_of: [:pending, :assigned, :in_progress, :review, :completed, :blocked]],
      default: :pending
    attribute :priority, :integer, default: 5
    attribute :complexity, :integer  # Fibonacci scale
    attribute :estimated_tokens, :integer
    attribute :actual_tokens, :integer
    attribute :artifacts, {:array, :string}, default: []
    attribute :context_requirements, :map
    attribute :success_criteria, {:array, :string}
  end

  relationships do
    belongs_to :workflow, Descartes.Orchestration.Workflow
    belongs_to :parent_task, __MODULE__
    has_many :subtasks, __MODULE__, destination_attribute: :parent_task_id
    belongs_to :assigned_agent, Descartes.Agents.Agent
    many_to_many :dependencies, __MODULE__,
      through: Descartes.Tasks.TaskDependency,
      source_attribute_on_join_resource: :dependent_id,
      destination_attribute_on_join_resource: :dependency_id
  end

  calculations do
    calculate :can_start?, :boolean do
      Descartes.Calculations.AllDependenciesComplete
    end
    
    calculate :progress_percentage, :integer do
      Descartes.Calculations.TaskProgress
    end
  end
end

# Knowledge Node Resource
defmodule Descartes.Knowledge.Node do
  use Ash.Resource,
    data_layer: AshPostgres.DataLayer

  attributes do
    uuid_primary_key :id
    attribute :type, :atom,
      constraints: [one_of: [:decision, :pattern, :solution, :problem, :code, :architecture]]
    attribute :title, :string, allow_nil?: false
    attribute :content, :text, allow_nil?: false
    attribute :embedding, :vector, constraints: [dimensions: 1536]
    attribute :metadata, :map, default: %{}
    attribute :tags, {:array, :string}, default: []
    attribute :usage_count, :integer, default: 0
    attribute :quality_score, :float, default: 0.0
    timestamps()
  end

  relationships do
    belongs_to :project, Descartes.Projects.Project
    belongs_to :created_by_agent, Descartes.Agents.Agent
    many_to_many :related_nodes, __MODULE__,
      through: Descartes.Knowledge.Edge
  end

  preparations do
    prepare Descartes.Preparations.ComputeEmbedding
  end

  actions do
    read :semantic_search do
      argument :query, :string, allow_nil?: false
      argument :limit, :integer, default: 10
      prepare Descartes.Preparations.VectorSearch
    end
    
    read :graph_traverse do
      argument :start_node_id, :uuid, allow_nil?: false
      argument :depth, :integer, default: 2
      prepare Descartes.Preparations.GraphTraversal
    end
  end
end
```

### 4.2 Supporting Resources

```elixir
# Tool Registry
defmodule Descartes.Tools.Tool do
  use Ash.Resource,
    data_layer: AshPostgres.DataLayer

  attributes do
    uuid_primary_key :id
    attribute :name, :atom, allow_nil?: false
    attribute :category, :atom
    attribute :description, :string
    attribute :parameters_schema, :map
    attribute :requires_approval, :boolean, default: false
    attribute :cost_per_use, :decimal
    attribute :implementation, :atom  # Module name
  end

  actions do
    defaults [:read]
    
    action :execute, :map do
      argument :agent_id, :uuid, allow_nil?: false
      argument :parameters, :map
      run Descartes.Tools.Executor
    end
  end
end

# Message/Communication
defmodule Descartes.Agents.Message do
  use Ash.Resource,
    data_layer: AshPostgres.DataLayer

  attributes do
    uuid_primary_key :id
    attribute :type, :atom,
      constraints: [one_of: [:system, :user, :assistant, :function, :broadcast]]
    attribute :content, :text
    attribute :metadata, :map
    timestamps()
  end

  relationships do
    belongs_to :from_agent, Descartes.Agents.Agent
    belongs_to :to_agent, Descartes.Agents.Agent
    belongs_to :workflow, Descartes.Orchestration.Workflow
  end
end

# Artifact Storage
defmodule Descartes.Storage.Artifact do
  use Ash.Resource,
    data_layer: AshPostgres.DataLayer

  attributes do
    uuid_primary_key :id
    attribute :type, :atom,
      constraints: [one_of: [:code, :document, :diagram, :data, :model]]
    attribute :path, :string, allow_nil?: false
    attribute :content_type, :string
    attribute :size_bytes, :integer
    attribute :checksum, :string
    attribute :s3_url, :string
    attribute :metadata, :map
    timestamps()
  end

  relationships do
    belongs_to :project, Descartes.Projects.Project
    belongs_to :created_by_agent, Descartes.Agents.Agent
    belongs_to :task, Descartes.Tasks.Task
  end
end
```

---

## 5. Phoenix LiveView UI Components

### 5.1 Dashboard LiveView

```elixir
defmodule DescartesWeb.DashboardLive do
  use DescartesWeb, :live_view
  
  @impl true
  def mount(_params, _session, socket) do
    if connected?(socket) do
      Phoenix.PubSub.subscribe(Descartes.PubSub, "orchestration:updates")
    end
    
    {:ok,
     socket
     |> assign(:projects, list_projects())
     |> assign(:active_workflows, list_active_workflows())
     |> assign(:agent_metrics, calculate_agent_metrics())
     |> assign(:selected_tab, :overview)}
  end
  
  @impl true
  def handle_info({:workflow_update, workflow}, socket) do
    {:noreply, update_workflow_in_socket(socket, workflow)}
  end
  
  @impl true
  def handle_event("create_project", %{"project" => params}, socket) do
    case Descartes.create_project(params) do
      {:ok, project} ->
        {:noreply,
         socket
         |> put_flash(:info, "Project created successfully")
         |> push_navigate(to: ~p"/projects/#{project.id}")}
         
      {:error, changeset} ->
        {:noreply, assign(socket, :changeset, changeset)}
    end
  end
end
```

### 5.2 Workflow Monitor LiveView

```elixir
defmodule DescartesWeb.WorkflowMonitorLive do
  use DescartesWeb, :live_view
  
  @impl true
  def mount(%{"id" => workflow_id}, _session, socket) do
    if connected?(socket) do
      Phoenix.PubSub.subscribe(Descartes.PubSub, "workflow:#{workflow_id}")
    end
    
    workflow = Descartes.get_workflow!(workflow_id, load: [:agents, :tasks])
    
    {:ok,
     socket
     |> assign(:workflow, workflow)
     |> assign(:agents, workflow.agents)
     |> assign(:task_graph, build_task_graph(workflow.tasks))
     |> assign(:logs, [])
     |> assign(:selected_agent, nil)}
  end
  
  @impl true
  def handle_info({:agent_status, agent_update}, socket) do
    {:noreply, update_agent_in_socket(socket, agent_update)}
  end
  
  @impl true
  def handle_info({:log_entry, log}, socket) do
    {:noreply, assign(socket, :logs, [log | socket.assigns.logs])}
  end
  
  @impl true
  def handle_event("pause_workflow", _, socket) do
    Descartes.pause_workflow(socket.assigns.workflow)
    {:noreply, socket}
  end
  
  @impl true
  def handle_event("select_agent", %{"id" => agent_id}, socket) do
    agent = Enum.find(socket.assigns.agents, & &1.id == agent_id)
    {:noreply, assign(socket, :selected_agent, agent)}
  end
end
```

### 5.3 Knowledge Graph Explorer

```elixir
defmodule DescartesWeb.KnowledgeGraphLive do
  use DescartesWeb, :live_view
  
  @impl true
  def mount(_params, _session, socket) do
    {:ok,
     socket
     |> assign(:search_query, "")
     |> assign(:graph_data, nil)
     |> assign(:selected_node, nil)
     |> assign(:view_mode, :graph)}  # :graph, :list, :semantic
  end
  
  @impl true
  def handle_event("search", %{"query" => query}, socket) do
    nodes = Descartes.Knowledge.semantic_search(query, limit: 20)
    graph_data = build_graph_visualization(nodes)
    
    {:noreply,
     socket
     |> assign(:search_query, query)
     |> assign(:graph_data, graph_data)}
  end
  
  @impl true
  def handle_event("node_selected", %{"node_id" => node_id}, socket) do
    node = Descartes.Knowledge.get_node!(node_id, load: [:related_nodes])
    
    {:noreply,
     socket
     |> assign(:selected_node, node)
     |> push_event("highlight_node", %{id: node_id})}
  end
end
```

---

## 6. Orchestration Engine Implementation

### 6.1 Workflow Executor GenServer

```elixir
defmodule Descartes.Orchestration.WorkflowExecutor do
  use GenServer
  require Logger
  
  def start_link(workflow_id) do
    GenServer.start_link(__MODULE__, workflow_id, name: via_tuple(workflow_id))
  end
  
  def init(workflow_id) do
    workflow = Descartes.get_workflow!(workflow_id)
    
    state = %{
      workflow: workflow,
      agents: %{},
      task_queue: build_task_queue(workflow.tasks),
      completed_tasks: MapSet.new(),
      context: %{},
      metrics: init_metrics()
    }
    
    schedule_next_tick()
    {:ok, state}
  end
  
  def handle_info(:tick, state) do
    state = 
      state
      |> assign_ready_tasks()
      |> check_agent_status()
      |> update_metrics()
      |> broadcast_status()
    
    schedule_next_tick()
    {:noreply, state}
  end
  
  defp assign_ready_tasks(state) do
    ready_tasks = 
      state.task_queue
      |> Enum.filter(&task_ready?(&1, state.completed_tasks))
      |> Enum.take(available_agent_count(state))
    
    Enum.reduce(ready_tasks, state, fn task, acc ->
      assign_task_to_agent(acc, task)
    end)
  end
  
  defp task_ready?(task, completed_tasks) do
    task.dependencies
    |> Enum.all?(&MapSet.member?(completed_tasks, &1.id))
  end
end
```

### 6.2 Agent Supervisor

```elixir
defmodule Descartes.Agents.Supervisor do
  use DynamicSupervisor
  
  def start_link(init_arg) do
    DynamicSupervisor.start_link(__MODULE__, init_arg, name: __MODULE__)
  end
  
  def init(_init_arg) do
    DynamicSupervisor.init(strategy: :one_for_one)
  end
  
  def spawn_agent(agent_params) do
    spec = {Descartes.Agents.Worker, agent_params}
    DynamicSupervisor.start_child(__MODULE__, spec)
  end
  
  def terminate_agent(agent_pid) do
    DynamicSupervisor.terminate_child(__MODULE__, agent_pid)
  end
end

defmodule Descartes.Agents.Worker do
  use GenServer
  
  def init(agent_params) do
    agent = Descartes.create_agent!(agent_params)
    
    state = %{
      agent: agent,
      current_task: nil,
      conversation: [],
      token_count: 0,
      tools: load_available_tools(agent)
    }
    
    {:ok, state}
  end
  
  def handle_call({:assign_task, task}, _from, state) do
    result = execute_task(task, state)
    
    new_state = %{state | 
      current_task: task,
      conversation: result.messages,
      token_count: state.token_count + result.tokens_used
    }
    
    {:reply, :ok, new_state}
  end
  
  defp execute_task(task, state) do
    # Build prompt from task context
    prompt = build_task_prompt(task, state)
    
    # Call LLM API
    response = call_llm(prompt, state.agent.model, state.agent.temperature)
    
    # Parse and execute any tool calls
    handle_tool_calls(response, state)
    
    # Update knowledge graph
    extract_knowledge(task, response, state)
    
    # Return results
    %{
      messages: [prompt, response],
      tokens_used: count_tokens(prompt) + count_tokens(response),
      artifacts: extract_artifacts(response)
    }
  end
end
```

---

## 7. API Design

### 7.1 GraphQL API (via AshGraphql)

```elixir
defmodule Descartes.GraphqlSchema do
  use Ash.Api.GraphqlSchema,
    apis: [Descartes.Projects.Api, Descartes.Orchestration.Api]
  
  # Automatically generates:
  # - Queries: listProjects, getProject, listWorkflows, etc.
  # - Mutations: createProject, startPlanning, assignTask, etc.
  # - Subscriptions: workflowUpdates, agentStatus, taskProgress
end
```

### 7.2 REST API (via AshJsonApi)

```elixir
defmodule Descartes.JsonApiRouter do
  use AshJsonApi.Api.Router,
    apis: [Descartes.Projects.Api, Descartes.Orchestration.Api],
    json_schema: "/json_schema",
    open_api: "/open_api"
end
```

### 7.3 WebSocket Channels

```elixir
defmodule DescartesWeb.OrchestrationChannel do
  use DescartesWeb, :channel
  
  def join("orchestration:" <> workflow_id, _params, socket) do
    if authorized?(workflow_id, socket.assigns.user_id) do
      send(self(), :after_join)
      {:ok, assign(socket, :workflow_id, workflow_id)}
    else
      {:error, %{reason: "unauthorized"}}
    end
  end
  
  def handle_info(:after_join, socket) do
    push(socket, "initial_state", load_workflow_state(socket.assigns.workflow_id))
    {:noreply, socket}
  end
  
  def handle_in("pause_workflow", _, socket) do
    Descartes.pause_workflow(socket.assigns.workflow_id)
    broadcast(socket, "workflow_paused", %{})
    {:noreply, socket}
  end
  
  def handle_in("send_message_to_agent", %{"agent_id" => agent_id, "message" => msg}, socket) do
    Descartes.send_agent_message(agent_id, msg)
    {:noreply, socket}
  end
end
```

---

## 8. Development Phases

### Phase 1: Foundation (Weeks 1-4)
- [ ] Set up Ash + Phoenix project
- [ ] Define core resources (Project, Workflow, Agent, Task)
- [ ] Implement basic CRUD operations
- [ ] Set up PostgreSQL with pgvector
- [ ] Create authentication/authorization
- [ ] Deploy basic LiveView dashboard

### Phase 2: Orchestration Core (Weeks 5-8)
- [ ] Implement WorkflowExecutor GenServer
- [ ] Build Agent Supervisor tree
- [ ] Create task scheduling algorithm
- [ ] Implement LLM API integration
- [ ] Add basic tool execution framework
- [ ] Create real-time monitoring UI

### Phase 3: Intelligence Layer (Weeks 9-12)
- [ ] Implement Architect agent logic
- [ ] Build planning engine with DAG generation
- [ ] Create agent communication protocols
- [ ] Implement context sharing mechanisms
- [ ] Add knowledge extraction pipeline
- [ ] Build semantic search capabilities

### Phase 4: Advanced Features (Weeks 13-16)
- [ ] Add specialized agent types
- [ ] Implement advanced tool registry
- [ ] Create workflow templates
- [ ] Build knowledge graph visualization
- [ ] Add cost optimization algorithms
- [ ] Implement checkpoint/recovery

### Phase 5: Production Hardening (Weeks 17-20)
- [ ] Add comprehensive error handling
- [ ] Implement rate limiting and quotas
- [ ] Create backup/restore capabilities
- [ ] Add monitoring and alerting
- [ ] Performance optimization
- [ ] Security audit and fixes

---

## 9. Deployment Architecture

### 9.1 Cloud Deployment (Primary)

```yaml
# kubernetes/deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: descartes-orchestrator
spec:
  replicas: 3
  selector:
    matchLabels:
      app: descartes
  template:
    spec:
      containers:
      - name: app
        image: descartes:latest
        env:
        - name: PHX_HOST
          value: "descartes.ai"
        - name: DATABASE_URL
          valueFrom:
            secretKeyRef:
              name: descartes-secrets
              key: database_url
        resources:
          requests:
            memory: "512Mi"
            cpu: "500m"
          limits:
            memory: "2Gi"
            cpu: "2000m"
```

### 9.2 Alternative: Fly.io Deployment

```toml
# fly.toml
app = "descartes-orchestrator"

[env]
  PHX_HOST = "descartes.fly.dev"
  PORT = "8080"

[deploy]
  release_command = "/app/bin/descartes eval Descartes.Release.migrate"

[[services]]
  internal_port = 8080
  protocol = "tcp"
  
  [[services.ports]]
    handlers = ["http"]
    port = 80
  
  [[services.ports]]
    handlers = ["tls", "http"]
    port = 443
```

---

## 10. Configuration & Environment

### 10.1 Development Configuration

```elixir
# config/dev.exs
config :descartes, Descartes.Repo,
  username: "postgres",
  password: "postgres",
  database: "descartes_dev",
  hostname: "localhost",
  pool_size: 10

config :descartes, DescartesWeb.Endpoint,
  http: [port: 4000],
  debug_errors: true,
  code_reloader: true,
  check_origin: false,
  watchers: [
    esbuild: {Esbuild, :install_and_run, [:default, ~w(--sourcemap=inline --watch)]}
  ]

config :descartes, :llm,
  provider: :openai,
  api_key: System.get_env("OPENAI_API_KEY"),
  default_model: "gpt-4-turbo-preview",
  max_retries: 3,
  timeout: 60_000

config :descartes, :orchestration,
  max_concurrent_agents: 10,
  task_timeout_seconds: 300,
  checkpoint_interval_seconds: 60
```

### 10.2 Production Configuration

```elixir
# config/prod.exs
config :descartes, Descartes.Repo,
  ssl: true,
  pool_size: String.to_integer(System.get_env("POOL_SIZE") || "10"),
  socket_options: [:inet6]

config :descartes, DescartesWeb.Endpoint,
  http: [port: {:system, "PORT"}],
  url: [scheme: "https", host: System.get_env("PHX_HOST"), port: 443],
  cache_static_manifest: "priv/static/cache_manifest.json"

config :descartes, :llm,
  provider: :azure_openai,
  api_key: {:system, "AZURE_OPENAI_KEY"},
  endpoint: {:system, "AZURE_OPENAI_ENDPOINT"},
  default_model: "gpt-4-turbo",
  max_concurrent_calls: 50

config :descartes, :storage,
  adapter: :s3,
  bucket: {:system, "S3_BUCKET"},
  region: {:system, "AWS_REGION"}
```

---

## 11. Success Metrics & KPIs

### 11.1 Platform Metrics
- **Agent Efficiency**: Tasks completed per token spent
- **Parallel Execution Rate**: Average concurrent agents active
- **Success Rate**: % of workflows completed successfully
- **Time to Value**: Average time from project creation to first artifact

### 11.2 Performance Metrics
- **Response Time**: P50/P95/P99 latencies for API calls
- **Throughput**: Workflows processed per hour
- **Resource Utilization**: CPU/Memory usage per workflow
- **Error Rate**: Failed tasks / total tasks

### 11.3 Business Metrics
- **Cost per Project**: Total LLM tokens + compute costs
- **Knowledge Reuse Rate**: % of tasks leveraging existing knowledge
- **Developer Productivity**: Lines of code generated per hour
- **Quality Score**: Based on review feedback and bug rates

---

## 12. Risk Mitigation

### 12.1 Technical Risks
- **LLM API Failures**: Implement circuit breakers, fallback models
- **Cost Overruns**: Token budgets, automatic pausing at limits
- **Infinite Loops**: Timeout mechanisms, cycle detection
- **Data Loss**: Regular checkpointing, event sourcing

### 12.2 Operational Risks
- **Scaling Issues**: Auto-scaling policies, queue management
- **Security Breaches**: End-to-end encryption, audit logging
- **Compliance**: Data residency options, GDPR compliance

---

## 13. Future Enhancements

### Near-term (3-6 months)
- Multi-model support (Claude, Llama, Gemini)
- Custom agent training on organizational patterns
- IDE plugins (VSCode, JetBrains)
- Workflow marketplace

### Long-term (6-12 months)
- Self-improving agents via reinforcement learning
- Automated code review and refactoring
- Multi-team collaboration features
- Enterprise SSO and audit compliance
- On-premise deployment options

---

## Appendix A: Sample Workflow Definition

```elixir
defmodule Descartes.Workflows.SampleEcommerceProject do
  @workflow_template %{
    name: "E-commerce Platform Development",
    phases: [
      %{
        type: :architect,
        config: %{
          requirements: "Build a modern e-commerce platform with inventory management",
          constraints: ["Must use React", "PostgreSQL database", "Stripe payments"],
          deliverables: ["System architecture", "Database schema", "API design", "Task breakdown"]
        }
      },
      %{
        type: :planner,
        config: %{
          optimization_goal: :parallel_execution,
          time_budget: "2 weeks",
          token_budget: 1_000_000
        }
      },
      %{
        type: :executor,
        config: %{
          agent_allocation: %{
            backend: 3,
            frontend: 2,
            testing: 1,
            documentation: 1
          },
          quality_gates: ["Code review", "Test coverage > 80%", "Documentation complete"]
        }
      }
    ]
  }
end
```

---

This PRD provides a comprehensive blueprint for building Descartes as a cloud-native AI orchestration platform using Ash and Phoenix. The declarative nature of Ash dramatically reduces development time while Phoenix LiveView enables real-time monitoring without frontend complexity. The architecture supports both cloud deployment for long-running processes and maintains the option for local development workflows.
