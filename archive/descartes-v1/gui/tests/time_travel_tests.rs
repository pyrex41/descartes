/// Unit Tests for GUI Time Travel Component
///
/// This test suite validates the time travel debugging interface including:
/// - Event navigation (next, prev, jump)
/// - Playback controls (play, pause, speed, loop)
/// - Snapshot navigation
/// - Time range calculations
/// - Zoom and scroll operations
/// - State creation and management
/// - Event visibility and selection
use descartes_core::{AgentHistoryEvent, HistoryEventType, HistorySnapshot};
use descartes_gui::time_travel::{update, PlaybackState, TimeTravelMessage, TimeTravelState};
use serde_json::json;
use uuid::Uuid;

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Generate sample events for testing
fn generate_sample_events(count: usize) -> Vec<AgentHistoryEvent> {
    let mut events = Vec::new();
    let base_timestamp = 1700000000i64; // Nov 2023

    for i in 0..count {
        let event = AgentHistoryEvent {
            event_id: Uuid::new_v4(),
            agent_id: "test-agent".to_string(),
            timestamp: base_timestamp + (i as i64 * 60), // Events 60 seconds apart
            event_type: match i % 4 {
                0 => HistoryEventType::Thought,
                1 => HistoryEventType::Action,
                2 => HistoryEventType::ToolUse,
                _ => HistoryEventType::StateChange,
            },
            event_data: json!({
                "index": i,
                "description": format!("Event {}", i)
            }),
            git_commit_hash: if i % 5 == 0 {
                Some(format!("commit{}", i))
            } else {
                None
            },
            session_id: Some("test-session".to_string()),
            parent_event_id: if i > 0 { Some(Uuid::new_v4()) } else { None },
            tags: vec![format!("tag{}", i % 3)],
            metadata: Some(json!({"test": true})),
        };
        events.push(event);
    }

    events
}

/// Generate sample snapshots for testing
fn generate_sample_snapshots(events: &[AgentHistoryEvent], count: usize) -> Vec<HistorySnapshot> {
    let mut snapshots = Vec::new();
    let step = if events.is_empty() {
        1
    } else {
        (events.len() / count.max(1)).max(1)
    };

    for i in 0..count {
        let event_idx = (i * step).min(events.len().saturating_sub(1));
        let timestamp = if event_idx < events.len() {
            events[event_idx].timestamp
        } else {
            1700000000 + (i as i64 * 300)
        };

        let snapshot = HistorySnapshot {
            snapshot_id: Uuid::new_v4(),
            agent_id: "test-agent".to_string(),
            timestamp,
            events: if event_idx < events.len() {
                vec![events[event_idx].clone()]
            } else {
                Vec::new()
            },
            git_commit: Some(format!("snapshot-commit-{}", i)),
            description: Some(format!("Snapshot {}", i)),
            metadata: Some(json!({"snapshot_index": i})),
            agent_state: Some(json!({"state": "active"})),
        };
        snapshots.push(snapshot);
    }

    snapshots
}

// ============================================================================
// STATE CREATION TESTS
// ============================================================================

#[test]
fn test_state_creation() {
    let state = TimeTravelState::default();

    assert!(state.events.is_empty());
    assert!(state.snapshots.is_empty());
    assert_eq!(state.selected_index, None);
    assert!(!state.playback.playing);
    assert_eq!(state.playback.speed, 1.0);
    assert!(!state.playback.loop_enabled);
    assert!(!state.loading);
    assert_eq!(state.agent_id, None);
    assert_eq!(state.zoom_level, 1.0);
    assert_eq!(state.scroll_offset, 0);
}

#[test]
fn test_state_with_events() {
    let events = generate_sample_events(10);
    let mut state = TimeTravelState::default();
    state.events = events.clone();

    assert_eq!(state.events.len(), 10);
    assert_eq!(state.events[0].event_data["index"], 0);
    assert_eq!(state.events[9].event_data["index"], 9);
}

// ============================================================================
// EVENT NAVIGATION TESTS
// ============================================================================

#[test]
fn test_event_navigation() {
    let events = generate_sample_events(5);
    let mut state = TimeTravelState::default();
    state.events = events.clone();

    // Initially no selection
    assert_eq!(state.selected_index, None);

    // Jump to event 0
    state.jump_to_event(0);
    assert_eq!(state.selected_index, Some(0));

    // Next event
    state.next_event();
    assert_eq!(state.selected_index, Some(1));

    // Next event again
    state.next_event();
    assert_eq!(state.selected_index, Some(2));

    // Previous event
    state.prev_event();
    assert_eq!(state.selected_index, Some(1));

    // Jump to last event
    state.jump_to_event(4);
    assert_eq!(state.selected_index, Some(4));

    // Try to go next at end (should stay at 4)
    state.next_event();
    assert_eq!(state.selected_index, Some(4));

    // Previous from end
    state.prev_event();
    assert_eq!(state.selected_index, Some(3));
}

#[test]
fn test_next_event_from_none() {
    let events = generate_sample_events(5);
    let mut state = TimeTravelState::default();
    state.events = events.clone();
    state.selected_index = None;

    // Next from None should select first event
    state.next_event();
    assert_eq!(state.selected_index, Some(0));
}

#[test]
fn test_prev_event_at_start() {
    let events = generate_sample_events(5);
    let mut state = TimeTravelState::default();
    state.events = events.clone();
    state.selected_index = Some(0);

    // Previous at start should stay at 0
    state.prev_event();
    assert_eq!(state.selected_index, Some(0));
}

#[test]
fn test_jump_to_event_out_of_bounds() {
    let events = generate_sample_events(5);
    let mut state = TimeTravelState::default();
    state.events = events.clone();

    // Jump to invalid index should not change selection
    state.selected_index = Some(2);
    state.jump_to_event(100);
    assert_eq!(state.selected_index, Some(2)); // Should stay at 2
}

#[test]
fn test_jump_to_event_adjusts_scroll() {
    let events = generate_sample_events(200);
    let mut state = TimeTravelState::default();
    state.events = events.clone();
    state.zoom_level = 1.0;
    state.scroll_offset = 0;

    // Jump to event far ahead
    state.jump_to_event(150);
    assert_eq!(state.selected_index, Some(150));
    // Scroll should be adjusted to keep event visible
    assert!(state.scroll_offset > 0);
}

// ============================================================================
// PLAYBACK CONTROL TESTS
// ============================================================================

#[test]
fn test_playback_controls() {
    let events = generate_sample_events(10);
    let mut state = TimeTravelState::default();
    state.events = events.clone();

    // Initially not playing
    assert!(!state.playback.playing);

    // Toggle playback on
    update(&mut state, TimeTravelMessage::TogglePlayback);
    assert!(state.playback.playing);

    // Toggle playback off
    update(&mut state, TimeTravelMessage::TogglePlayback);
    assert!(!state.playback.playing);

    // Set speed
    update(&mut state, TimeTravelMessage::SetPlaybackSpeed(2.0));
    assert_eq!(state.playback.speed, 2.0);

    // Set different speed
    update(&mut state, TimeTravelMessage::SetPlaybackSpeed(0.5));
    assert_eq!(state.playback.speed, 0.5);

    // Toggle loop
    assert!(!state.playback.loop_enabled);
    update(&mut state, TimeTravelMessage::ToggleLoop);
    assert!(state.playback.loop_enabled);
    update(&mut state, TimeTravelMessage::ToggleLoop);
    assert!(!state.playback.loop_enabled);
}

#[test]
fn test_playback_pause_resume() {
    let mut state = TimeTravelState::default();
    state.events = generate_sample_events(5);

    // Start playback
    update(&mut state, TimeTravelMessage::TogglePlayback);
    assert!(state.playback.playing);

    // Pause
    update(&mut state, TimeTravelMessage::TogglePlayback);
    assert!(!state.playback.playing);

    // Resume
    update(&mut state, TimeTravelMessage::TogglePlayback);
    assert!(state.playback.playing);
}

#[test]
fn test_playback_advance() {
    let events = generate_sample_events(5);
    let mut state = TimeTravelState::default();
    state.events = events.clone();
    state.selected_index = Some(0);
    state.playback.playing = true;
    state.playback.loop_enabled = false;

    // Playback tick advances to next event
    update(&mut state, TimeTravelMessage::PlaybackTick);
    assert_eq!(state.selected_index, Some(1));

    update(&mut state, TimeTravelMessage::PlaybackTick);
    assert_eq!(state.selected_index, Some(2));
}

#[test]
fn test_playback_stops_at_end() {
    let events = generate_sample_events(3);
    let mut state = TimeTravelState::default();
    state.events = events.clone();
    state.selected_index = Some(2); // Last event
    state.playback.playing = true;
    state.playback.loop_enabled = false;

    // Playback tick at end should stop playback
    update(&mut state, TimeTravelMessage::PlaybackTick);
    assert!(!state.playback.playing);
    assert_eq!(state.selected_index, Some(2)); // Stays at last event
}

#[test]
fn test_playback_loops() {
    let events = generate_sample_events(3);
    let mut state = TimeTravelState::default();
    state.events = events.clone();
    state.selected_index = Some(2); // Last event
    state.playback.playing = true;
    state.playback.loop_enabled = true;

    // Playback tick at end with loop enabled should wrap to start
    update(&mut state, TimeTravelMessage::PlaybackTick);
    assert!(state.playback.playing); // Still playing
    assert_eq!(state.selected_index, Some(0)); // Wrapped to start
}

// ============================================================================
// SNAPSHOT NAVIGATION TESTS
// ============================================================================

#[test]
fn test_snapshot_navigation() {
    let events = generate_sample_events(10);
    let snapshots = generate_sample_snapshots(&events, 3);
    let mut state = TimeTravelState::default();
    state.events = events.clone();
    state.snapshots = snapshots.clone();

    // Jump to first snapshot
    let snapshot_id = snapshots[0].snapshot_id;
    state.jump_to_snapshot(&snapshot_id);

    // Should jump to event closest to snapshot timestamp
    assert!(state.selected_index.is_some());
    let selected_event = &state.events[state.selected_index.unwrap()];

    // The selected event's timestamp should be close to snapshot's timestamp
    let time_diff = (selected_event.timestamp - snapshots[0].timestamp).abs();
    assert!(time_diff <= 60); // Within 60 seconds
}

#[test]
fn test_jump_to_nonexistent_snapshot() {
    let events = generate_sample_events(5);
    let snapshots = generate_sample_snapshots(&events, 2);
    let mut state = TimeTravelState::default();
    state.events = events.clone();
    state.snapshots = snapshots.clone();
    state.selected_index = Some(2);

    // Jump to non-existent snapshot
    let fake_id = Uuid::new_v4();
    state.jump_to_snapshot(&fake_id);

    // Should not change selection
    assert_eq!(state.selected_index, Some(2));
}

#[test]
fn test_snapshot_with_update_message() {
    let events = generate_sample_events(10);
    let snapshots = generate_sample_snapshots(&events, 3);
    let mut state = TimeTravelState::default();
    state.events = events.clone();
    state.snapshots = snapshots.clone();

    let snapshot_id = snapshots[1].snapshot_id;
    update(&mut state, TimeTravelMessage::JumpToSnapshot(snapshot_id));

    assert!(state.selected_index.is_some());
}

// ============================================================================
// TIME RANGE TESTS
// ============================================================================

#[test]
fn test_time_range() {
    let events = generate_sample_events(5);
    let mut state = TimeTravelState::default();
    state.events = events.clone();

    let range = state.time_range();
    assert!(range.is_some());

    let (start, end) = range.unwrap();
    assert!(start < end);
    assert_eq!(start, events[0].timestamp);
    assert_eq!(end, events[4].timestamp);
}

#[test]
fn test_time_range_empty() {
    let state = TimeTravelState::default();
    let range = state.time_range();
    assert!(range.is_none());
}

#[test]
fn test_time_range_single_event() {
    let events = generate_sample_events(1);
    let mut state = TimeTravelState::default();
    state.events = events.clone();

    let range = state.time_range();
    assert!(range.is_some());

    let (start, end) = range.unwrap();
    assert_eq!(start, end);
    assert_eq!(start, events[0].timestamp);
}

#[test]
fn test_time_range_with_unordered_timestamps() {
    let mut state = TimeTravelState::default();

    // Create events with timestamps not in order
    let event1 = AgentHistoryEvent {
        event_id: Uuid::new_v4(),
        agent_id: "test".to_string(),
        timestamp: 1000,
        event_type: HistoryEventType::Thought,
        event_data: json!({}),
        git_commit_hash: None,
        session_id: None,
        parent_event_id: None,
        tags: Vec::new(),
        metadata: None,
    };

    let event2 = AgentHistoryEvent {
        timestamp: 500,
        ..event1.clone()
    };

    let event3 = AgentHistoryEvent {
        timestamp: 1500,
        ..event1.clone()
    };

    state.events = vec![event1, event2, event3];

    let range = state.time_range();
    assert!(range.is_some());

    let (start, end) = range.unwrap();
    assert_eq!(start, 500);
    assert_eq!(end, 1500);
}

// ============================================================================
// ZOOM AND SCROLL TESTS
// ============================================================================

#[test]
fn test_zoom_scroll() {
    let events = generate_sample_events(100);
    let mut state = TimeTravelState::default();
    state.events = events.clone();
    state.zoom_level = 1.0;
    state.scroll_offset = 0;

    // Zoom in
    update(&mut state, TimeTravelMessage::ZoomIn);
    assert!(state.zoom_level > 1.0);
    let zoom_after_in = state.zoom_level;

    // Zoom out
    update(&mut state, TimeTravelMessage::ZoomOut);
    assert!(state.zoom_level < zoom_after_in);

    // Scroll timeline forward
    update(&mut state, TimeTravelMessage::ScrollTimeline(10));
    assert_eq!(state.scroll_offset, 10);

    // Scroll timeline backward
    update(&mut state, TimeTravelMessage::ScrollTimeline(-5));
    assert_eq!(state.scroll_offset, 5);
}

#[test]
fn test_zoom_limits() {
    let mut state = TimeTravelState::default();
    state.zoom_level = 1.0;

    // Zoom in multiple times
    for _ in 0..20 {
        update(&mut state, TimeTravelMessage::ZoomIn);
    }
    assert!(state.zoom_level <= 10.0); // Should be capped at max

    // Zoom out multiple times
    for _ in 0..30 {
        update(&mut state, TimeTravelMessage::ZoomOut);
    }
    assert!(state.zoom_level >= 0.1); // Should be capped at min
}

#[test]
fn test_scroll_offset_clamping() {
    let events = generate_sample_events(20);
    let mut state = TimeTravelState::default();
    state.events = events.clone();
    state.scroll_offset = 0;

    // Scroll way past the end
    update(&mut state, TimeTravelMessage::ScrollTimeline(1000));
    assert!(state.scroll_offset <= state.events.len());

    // Scroll before the beginning
    update(&mut state, TimeTravelMessage::ScrollTimeline(-2000));
    assert_eq!(state.scroll_offset, 0);
}

#[test]
fn test_scroll_with_negative_delta() {
    let events = generate_sample_events(50);
    let mut state = TimeTravelState::default();
    state.events = events.clone();
    state.scroll_offset = 20;

    // Scroll backward
    update(&mut state, TimeTravelMessage::ScrollTimeline(-10));
    assert_eq!(state.scroll_offset, 10);

    // Scroll backward past start
    update(&mut state, TimeTravelMessage::ScrollTimeline(-20));
    assert_eq!(state.scroll_offset, 0);
}

// ============================================================================
// SELECTED EVENT TESTS
// ============================================================================

#[test]
fn test_selected_event() {
    let events = generate_sample_events(5);
    let mut state = TimeTravelState::default();
    state.events = events.clone();

    // No selection initially
    assert!(state.selected_event().is_none());

    // Select an event
    state.selected_index = Some(2);
    let selected = state.selected_event();
    assert!(selected.is_some());
    assert_eq!(selected.unwrap().event_data["index"], 2);
}

#[test]
fn test_selected_event_out_of_bounds() {
    let events = generate_sample_events(5);
    let mut state = TimeTravelState::default();
    state.events = events.clone();
    state.selected_index = Some(100);

    // Should return None for out of bounds
    assert!(state.selected_event().is_none());
}

#[test]
fn test_selected_timestamp() {
    let events = generate_sample_events(5);
    let mut state = TimeTravelState::default();
    state.events = events.clone();

    // No selection
    assert!(state.selected_timestamp().is_none());

    // With selection
    state.selected_index = Some(3);
    let timestamp = state.selected_timestamp();
    assert!(timestamp.is_some());
    assert_eq!(timestamp.unwrap(), events[3].timestamp);
}

// ============================================================================
// EVENTS IN VIEW TESTS
// ============================================================================

#[test]
fn test_events_in_view() {
    let events = generate_sample_events(200);
    let mut state = TimeTravelState::default();
    state.events = events.clone();
    state.zoom_level = 1.0;
    state.scroll_offset = 0;

    let visible = state.visible_events();
    assert!(!visible.is_empty());
    assert!(visible.len() <= 100); // Default events per screen
}

#[test]
fn test_events_in_view_with_zoom() {
    let events = generate_sample_events(200);
    let mut state = TimeTravelState::default();
    state.events = events.clone();
    state.zoom_level = 2.0; // More zoom = fewer events visible
    state.scroll_offset = 0;

    let visible = state.visible_events();
    assert!(visible.len() <= 50); // Fewer events at higher zoom
}

#[test]
fn test_events_in_view_with_scroll() {
    let events = generate_sample_events(200);
    let mut state = TimeTravelState::default();
    state.events = events.clone();
    state.zoom_level = 1.0;
    state.scroll_offset = 50;

    let visible = state.visible_events();
    assert!(!visible.is_empty());

    // First visible event should be at scroll offset
    if !visible.is_empty() {
        assert_eq!(visible[0].event_data["index"], 50);
    }
}

#[test]
fn test_events_in_view_at_end() {
    let events = generate_sample_events(20);
    let mut state = TimeTravelState::default();
    state.events = events.clone();
    state.zoom_level = 1.0;
    state.scroll_offset = 10;

    let visible = state.visible_events();
    assert_eq!(visible.len(), 10); // Should show remaining events
}

// ============================================================================
// UPDATE MESSAGE TESTS
// ============================================================================

#[test]
fn test_load_history_message() {
    let mut state = TimeTravelState::default();

    update(
        &mut state,
        TimeTravelMessage::LoadHistory("agent-123".to_string()),
    );

    assert!(state.loading);
    assert_eq!(state.agent_id, Some("agent-123".to_string()));
}

#[test]
fn test_history_loaded_message() {
    let mut state = TimeTravelState::default();
    state.loading = true;
    state.selected_index = Some(5);
    state.scroll_offset = 10;

    let events = generate_sample_events(20);
    let snapshots = generate_sample_snapshots(&events, 3);

    update(
        &mut state,
        TimeTravelMessage::HistoryLoaded(events.clone(), snapshots.clone()),
    );

    assert!(!state.loading);
    assert_eq!(state.events.len(), 20);
    assert_eq!(state.snapshots.len(), 3);
    assert_eq!(state.selected_index, None); // Reset on load
    assert_eq!(state.scroll_offset, 0); // Reset on load
}

#[test]
fn test_select_event_message() {
    let events = generate_sample_events(10);
    let mut state = TimeTravelState::default();
    state.events = events.clone();

    update(&mut state, TimeTravelMessage::SelectEvent(5));
    assert_eq!(state.selected_index, Some(5));
}

#[test]
fn test_select_timestamp_message() {
    let events = generate_sample_events(10);
    let mut state = TimeTravelState::default();
    state.events = events.clone();

    let target_timestamp = events[7].timestamp;
    update(
        &mut state,
        TimeTravelMessage::SelectTimestamp(target_timestamp),
    );

    assert_eq!(state.selected_index, Some(7));
}

#[test]
fn test_prev_next_event_messages() {
    let events = generate_sample_events(5);
    let mut state = TimeTravelState::default();
    state.events = events.clone();
    state.selected_index = Some(2);

    update(&mut state, TimeTravelMessage::NextEvent);
    assert_eq!(state.selected_index, Some(3));

    update(&mut state, TimeTravelMessage::PrevEvent);
    assert_eq!(state.selected_index, Some(2));
}

#[test]
fn test_timeline_slider_changed() {
    let events = generate_sample_events(100);
    let mut state = TimeTravelState::default();
    state.events = events.clone();

    // Slider at 0.0 should select first event
    update(&mut state, TimeTravelMessage::TimelineSliderChanged(0.0));
    assert_eq!(state.selected_index, Some(0));

    // Slider at 0.5 should select middle event
    update(&mut state, TimeTravelMessage::TimelineSliderChanged(0.5));
    assert_eq!(state.selected_index, Some(49));

    // Slider at 1.0 should select last event
    update(&mut state, TimeTravelMessage::TimelineSliderChanged(1.0));
    assert_eq!(state.selected_index, Some(99));
}

// ============================================================================
// EDGE CASES
// ============================================================================

#[test]
fn test_operations_on_empty_state() {
    let mut state = TimeTravelState::default();

    // These should not crash
    state.next_event();
    assert_eq!(state.selected_index, None);

    state.prev_event();
    assert_eq!(state.selected_index, None);

    assert!(state.selected_event().is_none());
    assert!(state.time_range().is_none());
    assert!(state.visible_events().is_empty());

    update(&mut state, TimeTravelMessage::PlaybackTick);
    update(&mut state, TimeTravelMessage::TogglePlayback);
}

#[test]
fn test_playback_state_defaults() {
    let playback = PlaybackState::default();

    assert!(!playback.playing);
    assert_eq!(playback.speed, 1.0);
    assert!(!playback.loop_enabled);
}

#[test]
fn test_complex_navigation_sequence() {
    let events = generate_sample_events(20);
    let mut state = TimeTravelState::default();
    state.events = events.clone();

    // Complex sequence of operations
    state.jump_to_event(10);
    assert_eq!(state.selected_index, Some(10));

    for _ in 0..3 {
        state.next_event();
    }
    assert_eq!(state.selected_index, Some(13));

    for _ in 0..5 {
        state.prev_event();
    }
    assert_eq!(state.selected_index, Some(8));

    state.jump_to_event(0);
    assert_eq!(state.selected_index, Some(0));

    state.prev_event(); // At start, should stay
    assert_eq!(state.selected_index, Some(0));

    state.jump_to_event(19);
    state.next_event(); // At end, should stay
    assert_eq!(state.selected_index, Some(19));
}

#[test]
fn test_timeline_hover_message() {
    let mut state = TimeTravelState::default();
    state.events = generate_sample_events(10);

    // Should not crash or change state
    update(&mut state, TimeTravelMessage::TimelineHover(Some(5)));
    update(&mut state, TimeTravelMessage::TimelineHover(None));

    // State should remain unchanged
    assert_eq!(state.selected_index, None);
}

#[test]
fn test_multiple_speed_changes() {
    let mut state = TimeTravelState::default();

    let speeds = vec![0.25, 0.5, 1.0, 2.0, 5.0, 10.0];
    for speed in speeds {
        update(&mut state, TimeTravelMessage::SetPlaybackSpeed(speed));
        assert_eq!(state.playback.speed, speed);
    }
}

#[test]
fn test_zoom_and_navigation_interaction() {
    let events = generate_sample_events(200);
    let mut state = TimeTravelState::default();
    state.events = events.clone();

    // Zoom in
    update(&mut state, TimeTravelMessage::ZoomIn);
    update(&mut state, TimeTravelMessage::ZoomIn);

    // Jump to event - should adjust scroll
    state.jump_to_event(100);
    assert_eq!(state.selected_index, Some(100));
    assert!(state.scroll_offset > 0);

    // Scroll should keep selected event visible
    let visible = state.visible_events();
    let visible_indices: Vec<usize> =
        (state.scroll_offset..state.scroll_offset + visible.len()).collect();
    assert!(visible_indices.contains(&100));
}
