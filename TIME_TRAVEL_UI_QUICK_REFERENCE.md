# Time Travel UI - Quick Reference Guide

## File Locations

```
/home/user/descartes/descartes/gui/src/time_travel.rs     (1,056 lines)
/home/user/descartes/descartes/gui/src/main.rs            (512 lines)
/home/user/descartes/descartes/gui/Cargo.toml             (Updated)
```

## Quick Start

### Running the Application

```bash
cd /home/user/descartes/descartes
cargo run -p descartes-gui
```

### Loading Demo Data

1. Launch application
2. Click "Debugger" in navigation
3. Click "Load Sample History"
4. Explore the timeline!

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| **←** | Previous event |
| **→** | Next event |
| **Space** | Play/Pause |
| **+** or **=** | Zoom in |
| **-** | Zoom out |
| **1** | 0.5x speed |
| **2** | 1x speed |
| **3** | 2x speed |
| **4** | 5x speed |
| **L** | Toggle loop |

## Component Overview

### TimeTravelState

Main state container:

```rust
use time_travel::TimeTravelState;

let mut state = TimeTravelState::default();
```

Key methods:
- `selected_event()` - Get current event
- `next_event()` - Move forward
- `prev_event()` - Move backward
- `jump_to_event(idx)` - Jump to index
- `time_range()` - Get min/max time

### UI View

```rust
use time_travel;

// In your view function
let time_travel_ui = time_travel::view(&state)
    .map(Message::TimeTravel);
```

### Message Handling

```rust
use time_travel::{TimeTravelMessage, update};

match msg {
    Message::TimeTravel(tt_msg) => {
        update(&mut self.time_travel_state, tt_msg);
    }
}
```

## Event Type Colors

```rust
Thought        => 💭 Blue
Action         => ⚡ Green
ToolUse        => 🔧 Orange
StateChange    => 🔄 Magenta
Communication  => 💬 Cyan
Decision       => 🎯 Yellow
Error          => ❌ Red
System         => ⚙ Gray
```

## Common Tasks

### Creating Sample Events

```rust
use descartes_core::{AgentHistoryEvent, HistoryEventType};
use serde_json::json;

let event = AgentHistoryEvent::new(
    "agent-id".to_string(),
    HistoryEventType::Action,
    json!({"action": "do_something"})
)
.with_git_commit("abc123".to_string())
.with_tags(vec!["important".to_string()]);
```

### Loading Events

```rust
use time_travel::TimeTravelMessage;

// Prepare your events
let events = vec![event1, event2, event3];
let snapshots = vec![snapshot1, snapshot2];

// Load into state
time_travel::update(
    &mut state,
    TimeTravelMessage::HistoryLoaded(events, snapshots)
);
```

### Navigating

```rust
// Programmatic navigation
time_travel::update(&mut state, TimeTravelMessage::NextEvent);
time_travel::update(&mut state, TimeTravelMessage::PrevEvent);
time_travel::update(&mut state, TimeTravelMessage::SelectEvent(5));
```

### Playback Control

```rust
// Start playback at 2x speed
time_travel::update(&mut state, TimeTravelMessage::SetPlaybackSpeed(2.0));
time_travel::update(&mut state, TimeTravelMessage::TogglePlayback);

// Enable looping
time_travel::update(&mut state, TimeTravelMessage::ToggleLoop);
```

## Integration Pattern

### 1. Add Module

```rust
mod time_travel;
use time_travel::{TimeTravelState, TimeTravelMessage};
```

### 2. Add State

```rust
struct MyApp {
    time_travel_state: TimeTravelState,
}
```

### 3. Add Message Variant

```rust
enum Message {
    TimeTravel(TimeTravelMessage),
}
```

### 4. Handle Messages

```rust
fn update(&mut self, msg: Message) {
    match msg {
        Message::TimeTravel(tt_msg) => {
            time_travel::update(&mut self.time_travel_state, tt_msg);
        }
    }
}
```

### 5. Render View

```rust
fn view(&self) -> Element<Message> {
    time_travel::view(&self.time_travel_state)
        .map(Message::TimeTravel)
}
```

### 6. Add Keyboard Support (Optional)

```rust
fn subscription(&self) -> Subscription<Message> {
    iced::event::listen_with(|event, _, _| {
        if let Event::Keyboard(KeyPressed { key, .. }) = event {
            match key {
                Key::Named(Named::ArrowLeft) => {
                    Some(Message::TimeTravel(TimeTravelMessage::PrevEvent))
                }
                // ... more keys
                _ => None
            }
        } else {
            None
        }
    })
}
```

## Customization

### Timeline Settings

```rust
state.timeline_settings.show_icons = true;
state.timeline_settings.show_git_commits = true;
state.timeline_settings.show_timestamps = true;
state.timeline_settings.height = 120.0;
state.timeline_settings.marker_size = 10.0;
```

### Playback Settings

```rust
state.playback.speed = 2.0;
state.playback.loop_enabled = true;
```

### Zoom and Scroll

```rust
state.zoom_level = 2.0;     // 2x zoom
state.scroll_offset = 10;   // Start at event 10
```

## Data Structures Reference

### TimeTravelState

```rust
pub struct TimeTravelState {
    pub events: Vec<AgentHistoryEvent>,
    pub snapshots: Vec<HistorySnapshot>,
    pub selected_index: Option<usize>,
    pub playback: PlaybackState,
    pub timeline_settings: TimelineSettings,
    pub loading: bool,
    pub agent_id: Option<String>,
    pub zoom_level: f32,
    pub scroll_offset: usize,
}
```

### TimeTravelMessage

```rust
pub enum TimeTravelMessage {
    // Loading
    LoadHistory(String),
    HistoryLoaded(Vec<AgentHistoryEvent>, Vec<HistorySnapshot>),

    // Navigation
    SelectEvent(usize),
    SelectTimestamp(i64),
    PrevEvent,
    NextEvent,
    JumpToSnapshot(uuid::Uuid),

    // Playback
    TogglePlayback,
    SetPlaybackSpeed(f32),
    ToggleLoop,
    PlaybackTick,

    // View
    ZoomIn,
    ZoomOut,
    ScrollTimeline(i32),
    TimelineSliderChanged(f32),
    TimelineHover(Option<usize>),
}
```

## Troubleshooting

### No events shown

- Check `state.events.is_empty()`
- Verify events loaded with `HistoryLoaded` message
- Check if loading flag is stuck: `state.loading`

### Timeline not visible

- Check `timeline_settings.height`
- Verify canvas bounds
- Check zoom level (might be too far out)

### Keyboard shortcuts not working

- Ensure subscription is registered
- Check current view mode
- Verify event handler matches

### Events not selectable

- Check `selected_index` updates
- Verify message routing
- Check bounds (index < events.len())

## Tips and Tricks

### Performance

- Keep `visible_events()` count reasonable
- Use zoom to reduce rendered events
- Batch event loading

### UX

- Start with first event selected
- Auto-scroll to keep selection visible
- Use appropriate speed for demo (1x or 2x)

### Development

- Use sample data for rapid testing
- Log state changes for debugging
- Test with various event counts

## API Reference

### View Function

```rust
pub fn view(state: &TimeTravelState) -> Element<TimeTravelMessage>
```

Renders the complete time travel UI.

### Update Function

```rust
pub fn update(state: &mut TimeTravelState, message: TimeTravelMessage)
```

Updates state based on message.

### Helper Functions

```rust
fn event_type_color(event_type: &HistoryEventType) -> Color
fn event_type_icon(event_type: &HistoryEventType) -> &'static str
```

## Examples

### Complete Integration Example

```rust
use iced::{Element, Subscription, Task};
use time_travel::{TimeTravelState, TimeTravelMessage, view, update};

struct App {
    tt_state: TimeTravelState,
}

#[derive(Clone)]
enum Message {
    TimeTravel(TimeTravelMessage),
    LoadData,
}

impl App {
    fn update(&mut self, msg: Message) -> Task<Message> {
        match msg {
            Message::TimeTravel(tt_msg) => {
                update(&mut self.tt_state, tt_msg);
            }
            Message::LoadData => {
                // Load your events
                let events = load_events();
                update(
                    &mut self.tt_state,
                    TimeTravelMessage::HistoryLoaded(events, vec![])
                );
            }
        }
        Task::none()
    }

    fn view(&self) -> Element<Message> {
        view(&self.tt_state).map(Message::TimeTravel)
    }

    fn subscription(&self) -> Subscription<Message> {
        // Add keyboard support
        keyboard_subscription()
    }
}
```

## Resources

- **Full Documentation**: `/home/user/descartes/PHASE3_7_4_TIME_TRAVEL_UI_IMPLEMENTATION.md`
- **Agent History Models**: `/home/user/descartes/descartes/core/src/agent_history.rs`
- **Iced Documentation**: https://docs.rs/iced/
- **Source Code**: `/home/user/descartes/descartes/gui/src/time_travel.rs`

## Support

For issues or questions:
1. Check the full implementation report
2. Review the source code comments
3. Test with sample data first
4. Verify integration steps

---

**Quick Reference Version**: 1.0
**Last Updated**: 2025-11-24
