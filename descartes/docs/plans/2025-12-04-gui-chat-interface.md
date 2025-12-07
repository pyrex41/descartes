# GUI Chat Interface Implementation Plan

## Overview

Add a chat/prompt interface to the Descartes GUI that allows users to interact with agents directly from the GUI, rather than requiring CLI usage. This is a critical missing piece - currently the GUI can only monitor agents but cannot send tasks/prompts to them.

## Current State Analysis

### What Exists Now
- **GUI Architecture**: Iced-based application with message-driven architecture (`gui/src/main.rs`)
- **ViewModes**: Sessions, Dashboard, TaskBoard, SwarmMonitor, Debugger, DagEditor, FileBrowser, KnowledgeGraph
- **RPC Client**: `GuiRpcClient` wrapper around `RpcClient` for daemon communication (`gui/src/rpc_client.rs`)
- **Text Input Patterns**: Used in session creation dialog (`session_selector.rs:342-360`)
- **Provider Backend**: CLI spawn command shows how to create backends and send requests (`cli/src/commands/spawn.rs`)

### What's Missing
- No chat/prompt input UI component
- No way to send tasks to agents from GUI
- No response display area
- No transcript viewing in GUI
- No message history state

### Key Discoveries
- `spawn.rs` shows the pattern: create `ModelRequest` with messages, tools, system prompt, then call `backend.complete()` or `backend.stream()`
- `GuiRpcClient` has `pause_agent`, `resume_agent`, `attach_request` but no `spawn_agent` or `send_task`
- The daemon's `RpcClient` (`daemon/src/client.rs:284-300`) has `spawn_agent` method
- Text inputs in Iced use `text_input` widget with `on_input` and `on_submit` handlers

## Desired End State

After this implementation:
1. Users can type prompts in the GUI and send them to agents
2. Agent responses stream back and display in the GUI
3. Conversation history is maintained and viewable
4. Users can select provider/model before sending
5. Transcripts are saved like CLI does

### Verification
- [ ] Can type a prompt in the GUI
- [ ] Clicking "Send" or pressing Enter submits the prompt
- [ ] Response appears in the chat area
- [ ] Multiple messages form a conversation
- [ ] Transcript is saved to `.scud/sessions/`

## What We're NOT Doing

- Multi-agent orchestration UI (spawn_session tool support)
- Tool approval dialogs (tools run automatically like CLI)
- Voice/audio input
- File attachment to prompts
- Real-time collaborative editing

## Implementation Approach

Create a new "Chat" ViewMode that integrates with the existing provider infrastructure from the CLI. We'll reuse the `create_backend` and `ModelRequest` patterns from `spawn.rs`, adapting them for async GUI usage.

## Phase 1: Chat State and Messages

### Overview
Create the foundational state management and message types for the chat interface.

### Changes Required:

#### 1.1 Create Chat State Module

**File**: `gui/src/chat_state.rs` (new file)
**Changes**: Create new module for chat state management

```rust
//! Chat interface state management

use descartes_core::{Message as ModelMessage, MessageRole, ToolLevel};
use uuid::Uuid;

/// State for the chat interface
#[derive(Debug, Clone, Default)]
pub struct ChatState {
    /// Current prompt input
    pub prompt_input: String,
    /// Conversation history
    pub messages: Vec<ChatMessage>,
    /// Selected provider
    pub provider: String,
    /// Selected model
    pub model: String,
    /// Current tool level
    pub tool_level: ToolLevel,
    /// Is currently waiting for response
    pub loading: bool,
    /// Error message if any
    pub error: Option<String>,
    /// Current session/transcript ID
    pub session_id: Option<Uuid>,
}

/// A single chat message
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub id: Uuid,
    pub role: ChatRole,
    pub content: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Message role in chat
#[derive(Debug, Clone, PartialEq)]
pub enum ChatRole {
    User,
    Assistant,
    System,
}

/// Messages for chat operations
#[derive(Debug, Clone)]
pub enum ChatMessage {
    /// Update prompt input text
    UpdatePrompt(String),
    /// Submit the current prompt
    SubmitPrompt,
    /// Change provider
    SelectProvider(String),
    /// Change model
    SelectModel(String),
    /// Change tool level
    SelectToolLevel(ToolLevel),
    /// Response chunk received (for streaming)
    ResponseChunk(String),
    /// Response completed
    ResponseComplete(String),
    /// Error occurred
    Error(String),
    /// Clear error
    ClearError,
    /// Clear conversation
    ClearConversation,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            provider: "grok".to_string(),
            model: "grok-4-1-fast-reasoning".to_string(),
            tool_level: ToolLevel::Orchestrator,
            ..Default::default()
        }
    }
}

/// Update chat state based on messages
pub fn update(state: &mut ChatState, message: ChatMessage) {
    match message {
        ChatMessage::UpdatePrompt(text) => {
            state.prompt_input = text;
        }
        ChatMessage::SubmitPrompt => {
            if !state.prompt_input.trim().is_empty() {
                state.loading = true;
                state.error = None;
            }
        }
        ChatMessage::SelectProvider(provider) => {
            state.provider = provider;
        }
        ChatMessage::SelectModel(model) => {
            state.model = model;
        }
        ChatMessage::SelectToolLevel(level) => {
            state.tool_level = level;
        }
        ChatMessage::ResponseChunk(chunk) => {
            // Append to last assistant message or create new one
            if let Some(last) = state.messages.last_mut() {
                if last.role == ChatRole::Assistant {
                    last.content.push_str(&chunk);
                    return;
                }
            }
            // Create new assistant message
            state.messages.push(ChatMessage {
                id: Uuid::new_v4(),
                role: ChatRole::Assistant,
                content: chunk,
                timestamp: chrono::Utc::now(),
            });
        }
        ChatMessage::ResponseComplete(_) => {
            state.loading = false;
        }
        ChatMessage::Error(err) => {
            state.error = Some(err);
            state.loading = false;
        }
        ChatMessage::ClearError => {
            state.error = None;
        }
        ChatMessage::ClearConversation => {
            state.messages.clear();
            state.session_id = None;
        }
    }
}
```

#### 1.2 Add Chat Module to Main

**File**: `gui/src/main.rs`
**Changes**: Add module declaration and imports

Add after line 27 (`mod theme;`):
```rust
mod chat_state;
```

Add to imports section:
```rust
use chat_state::{ChatMessage as ChatMsg, ChatState};
```

#### 1.3 Add Chat State to DescartesGui

**File**: `gui/src/main.rs`
**Changes**: Add chat state field to main struct

Add to `DescartesGui` struct (after line 95):
```rust
    /// Chat interface state
    chat_state: ChatState,
```

Initialize in `new()` function:
```rust
    chat_state: ChatState::new(),
```

#### 1.4 Add Chat ViewMode

**File**: `gui/src/main.rs`
**Changes**: Add Chat to ViewMode enum

```rust
enum ViewMode {
    Sessions,
    Dashboard,
    Chat,        // NEW
    TaskBoard,
    SwarmMonitor,
    Debugger,
    DagEditor,
    FileBrowser,
    KnowledgeGraph,
}
```

#### 1.5 Add Chat Message Variant

**File**: `gui/src/main.rs`
**Changes**: Add Chat message variant to Message enum

```rust
    /// Chat interface message
    Chat(ChatMsg),
```

### Success Criteria:

#### Automated Verification:
- [ ] Compiles without errors: `cargo build -p descartes-gui`
- [ ] No clippy warnings: `cargo clippy -p descartes-gui`

#### Manual Verification:
- [ ] N/A for this phase (no visible UI changes yet)

---

## Phase 2: Chat UI Component

### Overview
Create the chat view with prompt input, message display, and controls.

### Changes Required:

#### 2.1 Create Chat View Module

**File**: `gui/src/chat_view.rs` (new file)
**Changes**: Create chat UI component

```rust
//! Chat interface view component

use crate::chat_state::{ChatMessage, ChatRole, ChatState};
use crate::theme::{button_styles, colors, container_styles, fonts};
use iced::alignment::Vertical;
use iced::widget::{button, column, container, row, scrollable, text, text_input, Space};
use iced::widget::text_input::Id as TextInputId;
use iced::{Element, Length};

/// Render the chat view
pub fn view(state: &ChatState) -> Element<ChatMessage> {
    let title = text("Agent Chat")
        .size(24)
        .font(fonts::MONO_BOLD)
        .color(colors::TEXT_PRIMARY);

    let subtitle = text("Send tasks to agents")
        .size(14)
        .color(colors::TEXT_SECONDARY);

    // Provider/model selector row
    let controls_row = row![
        text("Provider:").size(12).color(colors::TEXT_MUTED),
        Space::with_width(8),
        text(&state.provider).size(12).color(colors::PRIMARY),
        Space::with_width(24),
        text("Model:").size(12).color(colors::TEXT_MUTED),
        Space::with_width(8),
        text(&state.model).size(12).color(colors::PRIMARY),
        Space::with_width(Length::Fill),
        button(text("Clear").size(12).color(colors::TEXT_MUTED))
            .on_press(ChatMessage::ClearConversation)
            .padding([4, 8])
            .style(button_styles::nav),
    ]
    .align_y(Vertical::Center);

    // Messages area
    let messages_content: Vec<Element<ChatMessage>> = state
        .messages
        .iter()
        .map(|msg| view_message(msg))
        .collect();

    let messages_area = if messages_content.is_empty() {
        container(
            column![
                text("No messages yet").size(14).color(colors::TEXT_MUTED),
                Space::with_height(8),
                text("Type a task below and press Enter or click Send")
                    .size(12)
                    .color(colors::TEXT_MUTED),
            ]
            .align_x(iced::alignment::Horizontal::Center)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
    } else {
        container(
            scrollable(column(messages_content).spacing(12))
                .height(Length::Fill)
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(12)
        .style(container_styles::card)
    };

    // Error display
    let error_section = if let Some(ref error) = state.error {
        container(
            row![
                text("Error: ").size(12).color(colors::ERROR),
                text(error).size(12).color(colors::ERROR),
                Space::with_width(Length::Fill),
                button(text("X").size(12).color(colors::ERROR))
                    .on_press(ChatMessage::ClearError)
                    .padding([2, 8])
                    .style(button_styles::nav),
            ]
            .align_y(Vertical::Center)
        )
        .padding([8, 12])
        .style(container_styles::badge_error)
    } else {
        container(Space::with_height(0))
    };

    // Input area
    let input_row = row![
        container(
            text_input("Type your task here...", &state.prompt_input)
                .id(TextInputId::new("chat-prompt"))
                .on_input(ChatMessage::UpdatePrompt)
                .on_submit(ChatMessage::SubmitPrompt)
                .padding([12, 16])
                .size(14)
        )
        .width(Length::Fill),
        Space::with_width(12),
        button(
            text(if state.loading { "Sending..." } else { "Send" })
                .size(14)
                .font(fonts::MONO_MEDIUM)
                .color(if state.loading { colors::TEXT_MUTED } else { colors::PRIMARY })
        )
        .on_press_maybe(if state.loading || state.prompt_input.trim().is_empty() {
            None
        } else {
            Some(ChatMessage::SubmitPrompt)
        })
        .padding([12, 24])
        .style(button_styles::primary),
    ]
    .align_y(Vertical::Center);

    // Loading indicator
    let loading_indicator = if state.loading {
        container(
            text("● Agent is thinking...").size(12).color(colors::PRIMARY)
        )
        .padding([8, 0])
    } else {
        container(Space::with_height(0))
    };

    column![
        title,
        Space::with_height(4),
        subtitle,
        Space::with_height(16),
        controls_row,
        Space::with_height(12),
        messages_area,
        Space::with_height(8),
        error_section,
        loading_indicator,
        Space::with_height(8),
        input_row,
    ]
    .spacing(0)
    .into()
}

/// Render a single chat message
fn view_message(msg: &crate::chat_state::ChatMessage) -> Element<ChatMessage> {
    let (icon, name, bg_style) = match msg.role {
        ChatRole::User => ("◆", "You", container_styles::card),
        ChatRole::Assistant => ("◎", "Agent", container_styles::panel),
        ChatRole::System => ("⚙", "System", container_styles::card),
    };

    let role_color = match msg.role {
        ChatRole::User => colors::PRIMARY,
        ChatRole::Assistant => colors::SUCCESS,
        ChatRole::System => colors::TEXT_MUTED,
    };

    let timestamp = msg.timestamp.format("%H:%M:%S").to_string();

    container(
        column![
            row![
                text(icon).size(12).color(role_color),
                Space::with_width(8),
                text(name).size(12).font(fonts::MONO_MEDIUM).color(role_color),
                Space::with_width(Length::Fill),
                text(timestamp).size(10).color(colors::TEXT_MUTED),
            ]
            .align_y(Vertical::Center),
            Space::with_height(8),
            text(&msg.content).size(14).color(colors::TEXT_PRIMARY),
        ]
    )
    .padding(12)
    .width(Length::Fill)
    .style(bg_style)
    .into()
}
```

#### 2.2 Add Chat View Module to Main

**File**: `gui/src/main.rs`
**Changes**: Add module declaration

```rust
mod chat_view;
```

#### 2.3 Add Chat to Navigation

**File**: `gui/src/main.rs`
**Changes**: Add Chat to nav items in `view_navigation()` (around line 1017)

```rust
    let nav_items = vec![
        (ViewMode::Sessions, "\u{25C6}", "Sessions"),    // ◆
        (ViewMode::Dashboard, "\u{2302}", "Dashboard"),  // ⌂
        (ViewMode::Chat, "\u{2709}", "Chat"),            // ✉ NEW
        (ViewMode::TaskBoard, "\u{2630}", "Tasks"),      // ☰
        // ... rest unchanged
    ];
```

#### 2.4 Add Chat View Content Handler

**File**: `gui/src/main.rs`
**Changes**: Add Chat case to `view_content()` match (around line 1089)

```rust
    ViewMode::Chat => self.view_chat(),
```

#### 2.5 Add view_chat Method

**File**: `gui/src/main.rs`
**Changes**: Add method to render chat view

```rust
    /// Chat view
    fn view_chat(&self) -> Element<Message> {
        chat_view::view(&self.chat_state).map(Message::Chat)
    }
```

### Success Criteria:

#### Automated Verification:
- [ ] Compiles without errors: `cargo build -p descartes-gui`
- [ ] No clippy warnings: `cargo clippy -p descartes-gui`

#### Manual Verification:
- [ ] "Chat" appears in navigation sidebar
- [ ] Clicking "Chat" shows the chat interface
- [ ] Can type in the prompt input field
- [ ] Clear button clears conversation
- [ ] Send button appears disabled when input is empty

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that the UI renders correctly before proceeding to Phase 3.

---

## Phase 3: Backend Integration

### Overview
Connect the chat UI to the actual model providers so prompts get sent and responses are received.

### Changes Required:

#### 3.1 Add Chat Message Handler

**File**: `gui/src/main.rs`
**Changes**: Add handler in `update()` function for Chat messages

```rust
            Message::Chat(msg) => {
                use chat_state::ChatMessage as ChatMsg;

                let task = match &msg {
                    ChatMsg::SubmitPrompt => {
                        // Get prompt before updating state
                        let prompt = self.chat_state.prompt_input.clone();
                        if prompt.trim().is_empty() {
                            return iced::Task::none();
                        }

                        // Add user message to state
                        self.chat_state.messages.push(chat_state::ChatMessageEntry {
                            id: uuid::Uuid::new_v4(),
                            role: chat_state::ChatRole::User,
                            content: prompt.clone(),
                            timestamp: chrono::Utc::now(),
                        });
                        self.chat_state.prompt_input.clear();

                        // Get config and create backend
                        let provider = self.chat_state.provider.clone();
                        let model = self.chat_state.model.clone();
                        let tool_level = self.chat_state.tool_level.clone();
                        let messages: Vec<_> = self.chat_state.messages.iter()
                            .map(|m| descartes_core::Message {
                                role: match m.role {
                                    chat_state::ChatRole::User => descartes_core::MessageRole::User,
                                    chat_state::ChatRole::Assistant => descartes_core::MessageRole::Assistant,
                                    chat_state::ChatRole::System => descartes_core::MessageRole::System,
                                },
                                content: m.content.clone(),
                            })
                            .collect();

                        // Spawn async task to send request
                        iced::Task::perform(
                            send_chat_request(provider, model, tool_level, messages),
                            |result| match result {
                                Ok(response) => Message::Chat(ChatMsg::ResponseComplete(response)),
                                Err(e) => Message::Chat(ChatMsg::Error(e)),
                            }
                        )
                    }
                    _ => iced::Task::none(),
                };

                chat_state::update(&mut self.chat_state, msg);
                task
            }
```

#### 3.2 Add send_chat_request Function

**File**: `gui/src/main.rs`
**Changes**: Add async function at bottom of file

```rust
/// Send a chat request to the model backend
async fn send_chat_request(
    provider: String,
    model: String,
    tool_level: descartes_core::ToolLevel,
    messages: Vec<descartes_core::Message>,
) -> Result<String, String> {
    use descartes_core::{ConfigManager, ModelBackend, ModelRequest, ProviderFactory, get_system_prompt, get_tools};
    use std::collections::HashMap;

    // Load config
    let mut config_manager = ConfigManager::load(None)
        .map_err(|e| format!("Failed to load config: {}", e))?;
    let _ = config_manager.load_from_env();
    let config = config_manager.config();

    // Build provider config
    let mut provider_config: HashMap<String, String> = HashMap::new();

    match provider.as_str() {
        "grok" => {
            let api_key = config.providers.grok.api_key.as_ref()
                .ok_or("Grok API key not configured")?;
            provider_config.insert("api_key".to_string(), api_key.clone());
            provider_config.insert("endpoint".to_string(), config.providers.grok.endpoint.clone());
        }
        "anthropic" => {
            let api_key = config.providers.anthropic.api_key.as_ref()
                .ok_or("Anthropic API key not configured")?;
            provider_config.insert("api_key".to_string(), api_key.clone());
            provider_config.insert("endpoint".to_string(), config.providers.anthropic.endpoint.clone());
        }
        "openai" => {
            let api_key = config.providers.openai.api_key.as_ref()
                .ok_or("OpenAI API key not configured")?;
            provider_config.insert("api_key".to_string(), api_key.clone());
            provider_config.insert("endpoint".to_string(), config.providers.openai.endpoint.clone());
        }
        _ => return Err(format!("Unknown provider: {}", provider)),
    }

    // Create backend
    let mut backend = ProviderFactory::create(&provider, provider_config)
        .map_err(|e| format!("Failed to create backend: {}", e))?;

    backend.initialize().await
        .map_err(|e| format!("Failed to initialize backend: {}", e))?;

    // Build request
    let tools = get_tools(tool_level);
    let system_prompt = get_system_prompt(tool_level);

    let request = ModelRequest {
        messages,
        model,
        max_tokens: Some(4096),
        temperature: Some(0.7),
        system_prompt: Some(system_prompt.to_string()),
        tools: Some(tools),
    };

    // Send request
    let response = backend.complete(request).await
        .map_err(|e| format!("Request failed: {}", e))?;

    Ok(response.content)
}
```

### Success Criteria:

#### Automated Verification:
- [ ] Compiles without errors: `cargo build -p descartes-gui`
- [ ] No clippy warnings: `cargo clippy -p descartes-gui`

#### Manual Verification:
- [ ] Type a simple prompt like "Hello world" and press Enter
- [ ] See loading indicator while waiting
- [ ] Response appears from agent
- [ ] Can send follow-up messages
- [ ] Error displays if API key not configured

**Implementation Note**: After completing this phase and all automated verification passes, pause here for manual confirmation from the human that chat actually works before proceeding to Phase 4.

---

## Phase 4: Transcript Saving

### Overview
Save conversation transcripts to disk like the CLI does.

### Changes Required:

#### 4.1 Add Transcript Integration

**File**: `gui/src/main.rs`
**Changes**: Modify `send_chat_request` to save transcripts

Add transcript saving after getting response:
```rust
    // Save transcript
    let sessions_dir = descartes_core::default_sessions_dir();
    if let Ok(mut transcript) = descartes_core::TranscriptWriter::new(
        &sessions_dir,
        &provider,
        &model,
        &messages.last().map(|m| m.content.as_str()).unwrap_or("chat"),
        None,
        Some("orchestrator"),
    ) {
        for msg in &messages {
            match msg.role {
                descartes_core::MessageRole::User => transcript.add_user_message(&msg.content),
                descartes_core::MessageRole::Assistant => transcript.add_assistant_message(&msg.content),
                _ => {}
            }
        }
        transcript.add_assistant_message(&response.content);
        let _ = transcript.save();
    }
```

### Success Criteria:

#### Automated Verification:
- [ ] Compiles without errors: `cargo build -p descartes-gui`

#### Manual Verification:
- [ ] Send a chat message
- [ ] Check `.scud/sessions/` for new transcript file
- [ ] Transcript contains conversation history

---

## Testing Strategy

### Unit Tests
- ChatState initialization
- Message update logic
- ChatRole conversions

### Integration Tests
- Full chat flow with mock backend
- Transcript file creation

### Manual Testing Steps
1. Start GUI with `cargo run -p descartes-gui`
2. Click "Chat" in navigation
3. Type "What is 2+2?" and press Enter
4. Verify response appears
5. Send follow-up "And what is that times 3?"
6. Verify conversation context is maintained
7. Check `.scud/sessions/` for transcript
8. Click "Clear" and verify messages are removed

## Performance Considerations

- Non-streaming mode for simplicity (streaming can be added later)
- Messages stored in memory, not persisted between sessions
- Async task ensures UI remains responsive during API calls

## Migration Notes

N/A - this is new functionality with no data migration needed.

## References

- CLI spawn implementation: `cli/src/commands/spawn.rs`
- GUI RPC client: `gui/src/rpc_client.rs`
- Daemon RPC client: `daemon/src/client.rs`
- Session selector patterns: `gui/src/session_selector.rs`
- Main GUI architecture: `gui/src/main.rs`
