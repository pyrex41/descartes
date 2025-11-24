# Implementation Guide: Interactive Views

## Quick Start Implementation

### 1. Task Board Component (Elm)

```elm
-- src/TaskBoard.elm
module TaskBoard exposing (Model, Msg, init, update, view, subscriptions)

import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (..)
import Html.Keyed as Keyed
import Json.Decode as Decode
import Json.Encode as Encode
import Dict exposing (Dict)
import WebSocket
import DragDrop
import Time
import Task.Extra as TaskE

-- MODEL

type alias Model =
    { tasks : Dict String Task
    , epics : Dict String Epic
    , viewMode : ViewMode
    , dragDrop : DragDrop.Model TaskId ColumnId
    , selection : List TaskId
    , filter : FilterSet
    , websocket : Maybe WebSocket.Connection
    , liveUpdates : List Update
    , hoveredTask : Maybe TaskId
    }

type ViewMode
    = KanbanView
    | TreeView
    | GraphView
    | TableView

type alias Task =
    { id : String
    , title : String
    , description : String
    , status : TaskStatus
    , complexity : Int
    , assignee : Maybe Assignee
    , dependencies : List String
    , epic : String
    , sessionId : Maybe String
    , isLocked : Bool
    , hasApprovals : Bool
    }

type Assignee
    = Human String
    | Agent AgentType String  -- type and session

type TaskStatus
    = Backlog
    | Todo
    | InProgress
    | Review
    | Done
    | Blocked

type AgentType
    = Claude
    | OpenCode
    | Codex
    | Custom String

-- INIT

init : String -> ( Model, Cmd Msg )
init wsUrl =
    ( { tasks = Dict.empty
      , epics = Dict.empty
      , viewMode = KanbanView
      , dragDrop = DragDrop.init
      , selection = []
      , filter = defaultFilter
      , websocket = Nothing
      , liveUpdates = []
      , hoveredTask = Nothing
      }
    , WebSocket.connect wsUrl
    )

-- UPDATE

type Msg
    = TasksLoaded (Result Http.Error (Dict String Task))
    | SelectTask TaskId
    | ToggleSelection TaskId
    | DragDropMsg (DragDrop.Msg TaskId ColumnId)
    | ChangeStatus TaskId TaskStatus
    | AssignAgent TaskId AgentType
    | AttachToSession TaskId
    | SetViewMode ViewMode
    | ApplyFilter FilterSet
    | WebSocketMessage String
    | HoverTask (Maybe TaskId)
    | RefreshTasks

update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        TasksLoaded (Ok tasks) ->
            ( { model | tasks = tasks }, Cmd.none )

        SelectTask taskId ->
            ( { model | selection = [ taskId ] }, Cmd.none )

        ToggleSelection taskId ->
            let
                newSelection =
                    if List.member taskId model.selection then
                        List.filter ((/=) taskId) model.selection
                    else
                        taskId :: model.selection
            in
            ( { model | selection = newSelection }, Cmd.none )

        DragDropMsg subMsg ->
            let
                ( newDragDrop, result ) =
                    DragDrop.update subMsg model.dragDrop

                cmd =
                    case result of
                        Just ( taskId, columnId ) ->
                            changeTaskStatus taskId (columnToStatus columnId)

                        Nothing ->
                            Cmd.none
            in
            ( { model | dragDrop = newDragDrop }, cmd )

        ChangeStatus taskId newStatus ->
            ( model, changeTaskStatus taskId newStatus )

        AssignAgent taskId agentType ->
            ( model, assignAgentToTask taskId agentType )

        WebSocketMessage message ->
            case decodeUpdate message of
                Ok update ->
                    ( applyUpdate update model, Cmd.none )

                Err _ ->
                    ( model, Cmd.none )

        HoverTask maybeTaskId ->
            ( { model | hoveredTask = maybeTaskId }, Cmd.none )

        _ ->
            ( model, Cmd.none )

-- VIEW

view : Model -> Html Msg
view model =
    div [ class "task-board" ]
        [ viewHeader model
        , viewFilters model.filter
        , case model.viewMode of
            KanbanView ->
                viewKanban model

            TreeView ->
                viewTree model

            GraphView ->
                viewGraph model

            TableView ->
                viewTable model
        ]

viewKanban : Model -> Html Msg
viewKanban model =
    div [ class "kanban-board" ]
        [ viewColumn model "backlog" Backlog
        , viewColumn model "todo" Todo
        , viewColumn model "in-progress" InProgress
        , viewColumn model "review" Review
        , viewColumn model "done" Done
        ]

viewColumn : Model -> String -> TaskStatus -> Html Msg
viewColumn model columnId status =
    let
        columnTasks =
            model.tasks
                |> Dict.values
                |> List.filter (\t -> t.status == status)
                |> List.filter (matchesFilter model.filter)
    in
    div
        [ class ("kanban-column " ++ columnId)
        , DragDrop.droppable (DragDropMsg) columnId
        ]
        [ div [ class "column-header" ]
            [ h3 [] [ text (statusToString status) ]
            , span [ class "task-count" ] [ text (String.fromInt (List.length columnTasks)) ]
            ]
        , div [ class "column-tasks" ]
            (List.map (viewTaskCard model) columnTasks)
        ]

viewTaskCard : Model -> Task -> Html Msg
viewTaskCard model task =
    div
        [ class "task-card"
        , classList
            [ ( "selected", List.member task.id model.selection )
            , ( "hovered", model.hoveredTask == Just task.id )
            , ( "locked", task.isLocked )
            , ( "has-agent", hasAgent task )
            , ( "high-complexity", task.complexity > 8 )
            , ( "blocked", isBlocked model.tasks task )
            ]
        , DragDrop.draggable (DragDropMsg) task.id
        , onClick (SelectTask task.id)
        , onDoubleClick (AttachToSession task.id)
        , onMouseEnter (HoverTask (Just task.id))
        , onMouseLeave (HoverTask Nothing)
        ]
        [ viewComplexityBadge task.complexity
        , div [ class "task-header" ]
            [ span [ class "task-id" ] [ text task.id ]
            , span [ class "task-title" ] [ text task.title ]
            ]
        , viewAssignee task.assignee
        , viewTaskIndicators task
        ]

viewComplexityBadge : Int -> Html Msg
viewComplexityBadge complexity =
    div
        [ class "complexity-badge"
        , class (complexityClass complexity)
        ]
        [ text (String.fromInt complexity) ]

viewAssignee : Maybe Assignee -> Html Msg
viewAssignee maybeAssignee =
    case maybeAssignee of
        Just (Human name) ->
            div [ class "assignee human" ]
                [ span [ class "icon" ] [ text "👤" ]
                , span [ class "name" ] [ text name ]
                ]

        Just (Agent agentType sessionId) ->
            div [ class "assignee agent" ]
                [ span [ class "icon" ] [ text (agentIcon agentType) ]
                , span [ class "name" ] [ text (agentName agentType) ]
                ]

        Nothing ->
            div [ class "assignee unassigned" ]
                [ span [ class "icon" ] [ text "⭕" ]
                , span [ class "name" ] [ text "Unassigned" ]
                ]

-- SUBSCRIPTIONS

subscriptions : Model -> Sub Msg
subscriptions model =
    Sub.batch
        [ WebSocket.listen WebSocketMessage
        , Time.every (30 * Time.second) (\_ -> RefreshTasks)
        ]

-- HELPER FUNCTIONS

changeTaskStatus : TaskId -> TaskStatus -> Cmd Msg
changeTaskStatus taskId status =
    Http.post
        { url = "/api/tasks/" ++ taskId ++ "/status"
        , body = Http.jsonBody (Encode.object [ ( "status", Encode.string (statusToString status) ) ])
        , expect = Http.expectWhatever (\_ -> RefreshTasks)
        }

assignAgentToTask : TaskId -> AgentType -> Cmd Msg
assignAgentToTask taskId agentType =
    Http.post
        { url = "/api/tasks/" ++ taskId ++ "/assign"
        , body = Http.jsonBody (Encode.object [ ( "agent", Encode.string (agentTypeToString agentType) ) ])
        , expect = Http.expectJson TaskAssigned decodeAssignment
        }
```

### 2. Swarm Monitor Component (Elm)

```elm
-- src/SwarmMonitor.elm
module SwarmMonitor exposing (Model, Msg, init, update, view)

import Html exposing (..)
import Html.Attributes exposing (..)
import Html.Events exposing (..)
import Svg exposing (svg, g, circle, line, text_)
import Svg.Attributes as SvgAttr
import Dict exposing (Dict)
import Process
import Task
import WebSocket
import Json.Decode as Decode
import Json.Encode as Encode

-- MODEL

type alias Model =
    { agents : Dict SessionId AgentNode
    , connections : List AgentConnection
    , selectedAgent : Maybe SessionId
    , attachedSession : Maybe SessionId
    , swarmStatus : SwarmStatus
    , resourceUsage : ResourceMetrics
    , approvalQueue : List ApprovalRequest
    , terminalOutput : Dict SessionId (List String)
    }

type alias AgentNode =
    { id : SessionId
    , tool : AgentTool
    , task : Maybe TaskInfo
    , status : AgentStatus
    , metrics : AgentMetrics
    , position : Position
    , expanded : Bool
    }

type alias Position =
    { x : Float, y : Float }

type AgentTool
    = ClaudeTool
    | OpenCodeTool
    | CodexTool

type AgentStatus
    = Starting
    | Running ElapsedTime
    | Paused String
    | AwaitingApproval
    | Error String
    | Idle

-- INIT

init : ( Model, Cmd Msg )
init =
    ( { agents = Dict.empty
      , connections = []
      , selectedAgent = Nothing
      , attachedSession = Nothing
      , swarmStatus = SwarmIdle
      , resourceUsage = defaultMetrics
      , approvalQueue = []
      , terminalOutput = Dict.empty
      }
    , Cmd.batch
        [ connectToSwarm
        , pollAgentStatus
        ]
    )

-- UPDATE

type Msg
    = AgentUpdate SessionId AgentUpdate
    | SelectAgent SessionId
    | AttachToAgent SessionId
    | DetachFromAgent
    | PauseAgent SessionId
    | ResumeAgent SessionId
    | RestartAgent SessionId
    | TerminateAgent SessionId
    | ScaleAgents AgentTool Int
    | HandleApproval ApprovalId Bool
    | TerminalInput SessionId String
    | TerminalOutput SessionId String
    | UpdatePositions (Dict SessionId Position)
    | ToggleAgentExpanded SessionId
    | StartSwarm SwarmConfig
    | StopSwarm
    | WebSocketMsg String

update : Msg -> Model -> ( Model, Cmd Msg )
update msg model =
    case msg of
        SelectAgent sessionId ->
            ( { model | selectedAgent = Just sessionId }, Cmd.none )

        AttachToAgent sessionId ->
            ( { model | attachedSession = Just sessionId }
            , attachToSession sessionId
            )

        PauseAgent sessionId ->
            ( model, sendAgentCommand sessionId "pause" )

        ResumeAgent sessionId ->
            ( model, sendAgentCommand sessionId "resume" )

        TerminalInput sessionId input ->
            ( model, sendToAgent sessionId input )

        TerminalOutput sessionId output ->
            let
                currentOutput =
                    Dict.get sessionId model.terminalOutput
                        |> Maybe.withDefault []

                newOutput =
                    List.take 1000 (output :: currentOutput)
            in
            ( { model | terminalOutput = Dict.insert sessionId newOutput model.terminalOutput }
            , Cmd.none
            )

        HandleApproval approvalId approved ->
            ( { model | approvalQueue = List.filter (\a -> a.id /= approvalId) model.approvalQueue }
            , sendApprovalDecision approvalId approved
            )

        _ ->
            ( model, Cmd.none )

-- VIEW

view : Model -> Html Msg
view model =
    div [ class "swarm-monitor" ]
        [ viewTopBar model.swarmStatus model.resourceUsage
        , div [ class "monitor-content" ]
            [ viewAgentTopology model
            , viewSelectedAgent model
            , viewApprovalQueue model.approvalQueue
            ]
        , viewControlPanel model
        ]

viewAgentTopology : Model -> Html Msg
viewAgentTopology model =
    div [ class "agent-topology" ]
        [ svg
            [ SvgAttr.viewBox "0 0 800 600"
            , SvgAttr.class "topology-svg"
            ]
            (List.concat
                [ viewConnections model.connections
                , viewAgentNodes model.agents
                ]
            )
        ]

viewAgentNodes : Dict SessionId AgentNode -> List (Svg Msg)
viewAgentNodes agents =
    agents
        |> Dict.values
        |> List.map viewAgentNode

viewAgentNode : AgentNode -> Svg Msg
viewAgentNode agent =
    g
        [ SvgAttr.transform ("translate(" ++ String.fromFloat agent.position.x ++ "," ++ String.fromFloat agent.position.y ++ ")")
        , SvgAttr.class "agent-node-svg"
        , Svg.Events.onClick (SelectAgent agent.id)
        , Svg.Events.onDoubleClick (AttachToAgent agent.id)
        ]
        [ circle
            [ SvgAttr.r "40"
            , SvgAttr.class (agentStatusClass agent.status)
            , SvgAttr.fill (agentColor agent.tool)
            ]
            []
        , text_
            [ SvgAttr.y "5"
            , SvgAttr.textAnchor "middle"
            , SvgAttr.class "agent-label"
            ]
            [ Svg.text (agentLabel agent) ]
        , if agent.status == AwaitingApproval then
            circle
                [ SvgAttr.r "8"
                , SvgAttr.cx "30"
                , SvgAttr.cy "-30"
                , SvgAttr.fill "orange"
                , SvgAttr.class "approval-indicator"
                ]
                []
          else
            g [] []
        ]

viewSelectedAgent : Model -> Html Msg
viewSelectedAgent model =
    case model.selectedAgent of
        Just sessionId ->
            case Dict.get sessionId model.agents of
                Just agent ->
                    div [ class "agent-details" ]
                        [ viewAgentHeader agent
                        , viewAgentMetrics agent.metrics
                        , viewAgentActions agent
                        , if model.attachedSession == Just sessionId then
                            viewTerminal sessionId model.terminalOutput
                          else
                            text ""
                        ]

                Nothing ->
                    text ""

        Nothing ->
            div [ class "no-selection" ]
                [ text "Select an agent to view details" ]

viewTerminal : SessionId -> Dict SessionId (List String) -> Html Msg
viewTerminal sessionId outputs =
    div [ class "agent-terminal" ]
        [ div [ class "terminal-header" ]
            [ text ("Terminal - " ++ sessionId)
            , button [ onClick DetachFromAgent ] [ text "Detach" ]
            ]
        , div [ class "terminal-output" ]
            (outputs
                |> Dict.get sessionId
                |> Maybe.withDefault []
                |> List.map (\line -> div [ class "terminal-line" ] [ text line ])
            )
        , input
            [ class "terminal-input"
            , placeholder "Type command..."
            , onInput (TerminalInput sessionId)
            , onEnter (TerminalInput sessionId)
            ]
            []
        ]

viewControlPanel : Model -> Html Msg
viewControlPanel model =
    div [ class "control-panel" ]
        [ div [ class "control-section" ]
            [ h3 [] [ text "Swarm Control" ]
            , button
                [ onClick (if model.swarmStatus == SwarmRunning then StopSwarm else StartSwarm defaultConfig)
                , class (if model.swarmStatus == SwarmRunning then "stop-button" else "start-button")
                ]
                [ text (if model.swarmStatus == SwarmRunning then "Stop Swarm" else "Start Swarm") ]
            ]
        , div [ class "control-section" ]
            [ h3 [] [ text "Agent Scaling" ]
            , viewAgentScaler ClaudeTool (countAgents model.agents ClaudeTool)
            , viewAgentScaler OpenCodeTool (countAgents model.agents OpenCodeTool)
            , viewAgentScaler CodexTool (countAgents model.agents CodexTool)
            ]
        ]

viewAgentScaler : AgentTool -> Int -> Html Msg
viewAgentScaler tool count =
    div [ class "agent-scaler" ]
        [ span [] [ text (agentToolName tool ++ ": " ++ String.fromInt count) ]
        , button [ onClick (ScaleAgents tool (count - 1)) ] [ text "-" ]
        , button [ onClick (ScaleAgents tool (count + 1)) ] [ text "+" ]
        ]
```

### 3. Rust Backend for Real-time Updates

```rust
// src/dashboard/server.rs

use axum::{
    extract::{ws::WebSocket, WebSocketUpgrade, State},
    response::Response,
    routing::get,
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use uuid::Uuid;

#[derive(Clone)]
pub struct DashboardState {
    pub task_tx: broadcast::Sender<TaskUpdate>,
    pub agent_tx: broadcast::Sender<AgentUpdate>,
    pub sessions: Arc<RwLock<HashMap<Uuid, SessionState>>>,
    pub tasks: Arc<RwLock<HashMap<String, Task>>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum TaskUpdate {
    StatusChanged {
        task_id: String,
        old_status: TaskStatus,
        new_status: TaskStatus,
    },
    AgentAssigned {
        task_id: String,
        agent_type: String,
        session_id: Uuid,
    },
    TaskCompleted {
        task_id: String,
        duration: Duration,
    },
    DependencyMet {
        task_id: String,
        dependency_id: String,
    },
}

#[derive(Debug, Serialize, Deserialize)]
pub enum AgentUpdate {
    AgentStarted {
        session_id: Uuid,
        agent_type: String,
        task_id: Option<String>,
    },
    AgentStatusChanged {
        session_id: Uuid,
        status: AgentStatus,
    },
    MetricsUpdate {
        session_id: Uuid,
        metrics: AgentMetrics,
    },
    ApprovalRequested {
        session_id: Uuid,
        request: ApprovalRequest,
    },
    OutputReceived {
        session_id: Uuid,
        output: String,
        output_type: OutputType,
    },
}

pub async fn dashboard_websocket(
    ws: WebSocketUpgrade,
    State(state): State<DashboardState>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: DashboardState) {
    let (mut sender, mut receiver) = socket.split();
    
    // Subscribe to updates
    let mut task_rx = state.task_tx.subscribe();
    let mut agent_rx = state.agent_tx.subscribe();
    
    // Spawn task to send updates to client
    let state_clone = state.clone();
    let send_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                Ok(task_update) = task_rx.recv() => {
                    let msg = Message::Text(
                        serde_json::to_string(&json!({
                            "type": "task_update",
                            "data": task_update
                        })).unwrap()
                    );
                    if sender.send(msg).await.is_err() {
                        break;
                    }
                }
                Ok(agent_update) = agent_rx.recv() => {
                    let msg = Message::Text(
                        serde_json::to_string(&json!({
                            "type": "agent_update",
                            "data": agent_update
                        })).unwrap()
                    );
                    if sender.send(msg).await.is_err() {
                        break;
                    }
                }
            }
        }
    });
    
    // Handle incoming messages from client
    let state_clone2 = state.clone();
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            if let Message::Text(text) = msg {
                handle_client_message(&text, &state_clone2).await;
            }
        }
    });
    
    // Wait for either task to complete
    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }
}

async fn handle_client_message(text: &str, state: &DashboardState) {
    if let Ok(msg) = serde_json::from_str::<ClientMessage>(text) {
        match msg {
            ClientMessage::AttachToAgent { session_id } => {
                attach_to_agent_session(session_id, state).await;
            }
            ClientMessage::SendCommand { session_id, command } => {
                send_command_to_agent(session_id, command, state).await;
            }
            ClientMessage::PauseAgent { session_id } => {
                control_agent(session_id, AgentControl::Pause, state).await;
            }
            ClientMessage::ApproveRequest { request_id, approved } => {
                handle_approval(request_id, approved, state).await;
            }
            _ => {}
        }
    }
}

// Task board specific endpoints
pub async fn get_tasks(State(state): State<DashboardState>) -> Json<Vec<Task>> {
    let tasks = state.tasks.read().await;
    Json(tasks.values().cloned().collect())
}

pub async fn update_task_status(
    Path(task_id): Path<String>,
    Json(status): Json<TaskStatus>,
    State(state): State<DashboardState>,
) -> StatusCode {
    let mut tasks = state.tasks.write().await;
    
    if let Some(task) = tasks.get_mut(&task_id) {
        let old_status = task.status.clone();
        task.status = status.clone();
        
        // Broadcast update
        let _ = state.task_tx.send(TaskUpdate::StatusChanged {
            task_id,
            old_status,
            new_status: status,
        });
        
        StatusCode::OK
    } else {
        StatusCode::NOT_FOUND
    }
}

// Attach to agent's PTY session
async fn attach_to_agent_session(session_id: Uuid, state: &DashboardState) {
    let sessions = state.sessions.read().await;
    
    if let Some(session) = sessions.get(&session_id) {
        if let Some(pty) = &session.pty_handle {
            // Set up bidirectional streaming between WebSocket and PTY
            let (pty_tx, mut pty_rx) = pty.split();
            
            // Stream PTY output to dashboard
            tokio::spawn(async move {
                while let Some(output) = pty_rx.next().await {
                    let _ = state.agent_tx.send(AgentUpdate::OutputReceived {
                        session_id,
                        output: String::from_utf8_lossy(&output).to_string(),
                        output_type: OutputType::Stdout,
                    });
                }
            });
        }
    }
}

pub fn create_dashboard_router(state: DashboardState) -> Router {
    Router::new()
        .route("/ws", get(dashboard_websocket))
        .route("/api/tasks", get(get_tasks))
        .route("/api/tasks/:id/status", post(update_task_status))
        .route("/api/agents", get(get_agents))
        .route("/api/agents/:id/control", post(control_agent_endpoint))
        .with_state(state)
}
```

### 4. CSS Styling for Interactive Views

```css
/* src/styles/dashboard.css */

/* Task Board Styles */
.task-board {
    display: flex;
    flex-direction: column;
    height: 100vh;
    background: var(--bg-primary);
}

.kanban-board {
    display: flex;
    gap: 1rem;
    padding: 1rem;
    overflow-x: auto;
    flex: 1;
}

.kanban-column {
    flex: 1;
    min-width: 280px;
    background: var(--bg-secondary);
    border-radius: 8px;
    padding: 1rem;
    display: flex;
    flex-direction: column;
}

.task-card {
    background: var(--card-bg);
    border-radius: 6px;
    padding: 12px;
    margin-bottom: 8px;
    cursor: move;
    transition: all 0.2s;
    border-left: 4px solid transparent;
}

.task-card:hover {
    box-shadow: 0 4px 12px rgba(0,0,0,0.1);
    transform: translateY(-2px);
}

.task-card.selected {
    border-color: var(--accent-color);
    background: var(--selected-bg);
}

.task-card.has-agent {
    border-left-color: var(--ai-color);
}

.task-card.locked {
    opacity: 0.7;
    cursor: not-allowed;
}

.complexity-badge {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    width: 28px;
    height: 28px;
    border-radius: 50%;
    font-weight: bold;
    font-size: 12px;
}

.complexity-badge.low { background: #4caf50; color: white; }
.complexity-badge.medium { background: #ff9800; color: white; }
.complexity-badge.high { background: #f44336; color: white; }

/* Swarm Monitor Styles */
.swarm-monitor {
    display: grid;
    grid-template-rows: auto 1fr auto;
    height: 100vh;
    background: var(--bg-primary);
}

.agent-topology {
    background: var(--topology-bg);
    border-radius: 8px;
    margin: 1rem;
    position: relative;
    height: 500px;
}

.agent-node-svg {
    cursor: pointer;
    transition: all 0.3s;
}

.agent-node-svg:hover circle {
    stroke-width: 3;
    stroke: var(--accent-color);
}

.agent-node-svg.running circle {
    animation: pulse 2s infinite;
}

@keyframes pulse {
    0% { opacity: 1; }
    50% { opacity: 0.7; }
    100% { opacity: 1; }
}

.agent-terminal {
    background: #1e1e1e;
    color: #d4d4d4;
    font-family: 'Monaco', 'Menlo', monospace;
    font-size: 13px;
    border-radius: 6px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    height: 400px;
}

.terminal-output {
    flex: 1;
    overflow-y: auto;
    padding: 1rem;
    white-space: pre-wrap;
    word-break: break-all;
}

.terminal-line {
    margin: 2px 0;
    line-height: 1.4;
}

.terminal-line.stdout { color: #d4d4d4; }
.terminal-line.stderr { color: #f48771; }
.terminal-line.thinking { color: #7ca9ef; font-style: italic; }
.terminal-line.tool-call { color: #b5cea8; }

.terminal-input {
    background: #2d2d2d;
    border: none;
    color: #d4d4d4;
    padding: 0.5rem 1rem;
    font-family: inherit;
    font-size: inherit;
}

/* Control Panel */
.control-panel {
    background: var(--panel-bg);
    padding: 1rem;
    display: flex;
    gap: 2rem;
    border-top: 1px solid var(--border-color);
}

.agent-scaler {
    display: flex;
    align-items: center;
    gap: 0.5rem;
}

.agent-scaler button {
    width: 24px;
    height: 24px;
    border-radius: 4px;
    border: 1px solid var(--border-color);
    background: var(--button-bg);
    cursor: pointer;
}

/* Dark Theme Variables */
:root {
    --bg-primary: #1e1e1e;
    --bg-secondary: #252526;
    --card-bg: #2d2d30;
    --selected-bg: #3e3e42;
    --topology-bg: #1a1a1a;
    --panel-bg: #2d2d30;
    --text-primary: #d4d4d4;
    --text-secondary: #969696;
    --accent-color: #007acc;
    --ai-color: #4caf50;
    --border-color: #464647;
    --button-bg: #3e3e42;
}

/* Responsive Design */
@media (max-width: 768px) {
    .kanban-board {
        flex-direction: column;
    }
    
    .kanban-column {
        min-width: 100%;
    }
    
    .monitor-content {
        flex-direction: column;
    }
}
```

### 5. Integration Script

```bash
#!/bin/bash
# scripts/setup-dashboard.sh

echo "Setting up Descartes Interactive Dashboard..."

# Install Elm dependencies
echo "Installing Elm packages..."
cd frontend
elm install elm/html
elm install elm/http
elm install elm/json
elm install elm/time
elm install elm/svg
elm install elm-community/list-extra
elm install zaboco/elm-draggable

# Build Elm application
echo "Building Elm frontend..."
elm make src/Main.elm --output=public/dashboard.js --optimize

# Install Rust dependencies
echo "Adding Rust dependencies..."
cd ../backend
cargo add axum
cargo add tokio-tungstenite
cargo add uuid
cargo add serde_json

# Build Rust backend
echo "Building Rust backend..."
cargo build --release

# Create systemd service for dashboard
echo "Creating systemd service..."
cat > /etc/systemd/system/descartes-dashboard.service << EOF
[Unit]
Description=Descartes Dashboard Server
After=network.target

[Service]
Type=simple
User=$USER
WorkingDirectory=$(pwd)
ExecStart=$(pwd)/target/release/descartes-dashboard
Restart=on-failure

[Install]
WantedBy=multi-user.target
EOF

# Start services
systemctl daemon-reload
systemctl enable descartes-dashboard
systemctl start descartes-dashboard

echo "Dashboard available at http://localhost:8080"
echo "WebSocket endpoint: ws://localhost:8080/ws"
```

---

## Quick Start Guide

### 1. Run the Dashboard

```bash
# Start the backend
$ cargo run --bin descartes-dashboard

# In another terminal, start the Elm dev server
$ elm reactor
# Or for production
$ elm make src/Main.elm --output=public/dashboard.js --optimize

# Open browser to http://localhost:8080
```

### 2. Connect to Live Sessions

```javascript
// In browser console or frontend code
const ws = new WebSocket('ws://localhost:8080/ws');

ws.onmessage = (event) => {
    const update = JSON.parse(event.data);
    console.log('Update received:', update);
};

// Attach to an agent
ws.send(JSON.stringify({
    type: 'attach_to_agent',
    session_id: 'abc-123'
}));
```

### 3. Integrate with SCUD

```rust
// In SCUD's task update handler
let dashboard_state = /* get from context */;

// When task status changes
dashboard_state.task_tx.send(TaskUpdate::StatusChanged {
    task_id: task.id.clone(),
    old_status: task.status.clone(),
    new_status: new_status,
}).await?;
```

This implementation provides the foundation for both interactive views with real-time updates, drag-and-drop functionality, and live terminal attachment to running agent sessions.
