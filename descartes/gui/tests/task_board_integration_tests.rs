//! End-to-End Integration Tests for Task Board Real-Time Updates
//!
//! These tests verify the complete flow from task events to UI updates:
//! - Task creation → UI update
//! - Task status change → Kanban column transition
//! - Task deletion → UI removal
//! - Concurrent task updates
//! - Filter/sort persistence during updates
//! - Edge cases (connection loss, reconnection, event backlog)

use chrono::Utc;
use descartes_core::{Task, TaskComplexity, TaskPriority, TaskStatus};
use descartes_daemon::{DescartesEvent, TaskEvent, TaskEventType};
use descartes_gui::task_board::{update, KanbanBoard, TaskBoardMessage, TaskBoardState, TaskSort};
use serde_json::json;
use uuid::Uuid;

/// Helper function to create a sample task
fn create_sample_task(
    status: TaskStatus,
    priority: TaskPriority,
    complexity: TaskComplexity,
) -> Task {
    Task {
        id: Uuid::new_v4(),
        title: "Test Task".to_string(),
        description: Some("Test description".to_string()),
        status,
        priority,
        complexity,
        assigned_to: None,
        dependencies: vec![],
        created_at: Utc::now().timestamp(),
        updated_at: Utc::now().timestamp(),
        metadata: None,
    }
}

/// Helper function to create a task event
fn create_task_event(
    task: &Task,
    event_type: TaskEventType,
    include_task_data: bool,
) -> DescartesEvent {
    let data = if include_task_data {
        json!({
            "task": serde_json::to_value(task).unwrap(),
            "change_type": match event_type {
                TaskEventType::Created => "created",
                TaskEventType::Progress => "updated",
                TaskEventType::Cancelled => "deleted",
                _ => "unknown",
            }
        })
    } else {
        json!({
            "change_type": match event_type {
                TaskEventType::Created => "created",
                TaskEventType::Progress => "updated",
                TaskEventType::Cancelled => "deleted",
                _ => "unknown",
            }
        })
    };

    DescartesEvent::TaskEvent(TaskEvent {
        id: Uuid::new_v4().to_string(),
        task_id: task.id.to_string(),
        agent_id: None,
        timestamp: Utc::now(),
        event_type,
        data,
    })
}

#[test]
fn test_task_creation_updates_ui() {
    // Setup
    let mut state = TaskBoardState::new();
    assert_eq!(state.kanban_board.todo.len(), 0);

    // Create a task
    let task = create_sample_task(TaskStatus::Todo, TaskPriority::High, TaskComplexity::Simple);

    // Simulate receiving a task creation event
    let event = create_task_event(&task, TaskEventType::Created, true);
    update(&mut state, TaskBoardMessage::EventReceived(event));

    // Verify task appears in the board
    assert_eq!(state.kanban_board.todo.len(), 1);
    assert_eq!(state.kanban_board.todo[0].id, task.id);
    assert_eq!(state.kanban_board.todo[0].title, task.title);
}

#[test]
fn test_task_status_change_moves_column() {
    // Setup
    let mut state = TaskBoardState::new();

    // Create initial task in Todo
    let mut task = create_sample_task(TaskStatus::Todo, TaskPriority::High, TaskComplexity::Simple);
    update(&mut state, TaskBoardMessage::TaskCreated(task.clone()));

    assert_eq!(state.kanban_board.todo.len(), 1);
    assert_eq!(state.kanban_board.in_progress.len(), 0);

    // Update task status to InProgress
    task.status = TaskStatus::InProgress;
    task.updated_at = Utc::now().timestamp();

    let event = create_task_event(&task, TaskEventType::Progress, true);
    update(&mut state, TaskBoardMessage::EventReceived(event));

    // Verify task moved to InProgress column
    assert_eq!(state.kanban_board.todo.len(), 0);
    assert_eq!(state.kanban_board.in_progress.len(), 1);
    assert_eq!(state.kanban_board.in_progress[0].id, task.id);
}

#[test]
fn test_task_deletion_removes_from_ui() {
    // Setup
    let mut state = TaskBoardState::new();

    // Create a task
    let task = create_sample_task(
        TaskStatus::Done,
        TaskPriority::Medium,
        TaskComplexity::Moderate,
    );
    update(&mut state, TaskBoardMessage::TaskCreated(task.clone()));

    assert_eq!(state.kanban_board.done.len(), 1);

    // Delete task
    update(&mut state, TaskBoardMessage::TaskDeleted(task.id));

    // Verify task is removed
    assert_eq!(state.kanban_board.done.len(), 0);
}

#[test]
fn test_concurrent_task_updates() {
    // Setup
    let mut state = TaskBoardState::new();

    // Create multiple tasks rapidly
    let tasks: Vec<Task> = (0..10)
        .map(|i| {
            let status = match i % 4 {
                0 => TaskStatus::Todo,
                1 => TaskStatus::InProgress,
                2 => TaskStatus::Done,
                _ => TaskStatus::Blocked,
            };
            create_sample_task(status, TaskPriority::Medium, TaskComplexity::Moderate)
        })
        .collect();

    // Apply all tasks
    for task in &tasks {
        update(&mut state, TaskBoardMessage::TaskCreated(task.clone()));
    }

    // Verify all tasks are in correct columns
    let total_tasks = state.kanban_board.todo.len()
        + state.kanban_board.in_progress.len()
        + state.kanban_board.done.len()
        + state.kanban_board.blocked.len();

    assert_eq!(total_tasks, 10);

    // Verify correct distribution
    assert_eq!(state.kanban_board.todo.len(), 3); // indices 0, 4, 8
    assert_eq!(state.kanban_board.in_progress.len(), 3); // indices 1, 5, 9
    assert_eq!(state.kanban_board.done.len(), 2); // indices 2, 6
    assert_eq!(state.kanban_board.blocked.len(), 2); // indices 3, 7
}

#[test]
fn test_filter_persistence_during_updates() {
    // Setup
    let mut state = TaskBoardState::new();

    // Set filter for High priority
    update(
        &mut state,
        TaskBoardMessage::FilterByPriority(Some(TaskPriority::High)),
    );

    // Add tasks with different priorities
    let high_task =
        create_sample_task(TaskStatus::Todo, TaskPriority::High, TaskComplexity::Simple);
    let low_task = create_sample_task(TaskStatus::Todo, TaskPriority::Low, TaskComplexity::Simple);

    update(&mut state, TaskBoardMessage::TaskCreated(high_task.clone()));
    update(&mut state, TaskBoardMessage::TaskCreated(low_task.clone()));

    // Verify both tasks are in the board (filter applies during rendering, not storage)
    assert_eq!(state.kanban_board.todo.len(), 2);

    // Verify filter is still active
    assert_eq!(state.filters.priority, Some(TaskPriority::High));

    // Apply filter to get filtered view
    let filtered = state.apply_filters(state.kanban_board.todo.clone());
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].priority, TaskPriority::High);
}

#[test]
fn test_sort_persistence_during_updates() {
    // Setup
    let mut state = TaskBoardState::new();

    // Set sort order
    update(
        &mut state,
        TaskBoardMessage::ChangeSortOrder(TaskSort::Priority),
    );

    // Add tasks with different priorities
    let low_task = create_sample_task(TaskStatus::Todo, TaskPriority::Low, TaskComplexity::Simple);
    let critical_task = create_sample_task(
        TaskStatus::Todo,
        TaskPriority::Critical,
        TaskComplexity::Simple,
    );
    let medium_task = create_sample_task(
        TaskStatus::Todo,
        TaskPriority::Medium,
        TaskComplexity::Simple,
    );

    update(&mut state, TaskBoardMessage::TaskCreated(low_task.clone()));
    update(
        &mut state,
        TaskBoardMessage::TaskCreated(critical_task.clone()),
    );
    update(
        &mut state,
        TaskBoardMessage::TaskCreated(medium_task.clone()),
    );

    // Verify sort order is maintained
    assert_eq!(state.sort, TaskSort::Priority);

    // Apply sort
    let sorted = state.sort_tasks(state.kanban_board.todo.clone());
    assert_eq!(sorted[0].priority, TaskPriority::Critical);
    assert_eq!(sorted[1].priority, TaskPriority::Medium);
    assert_eq!(sorted[2].priority, TaskPriority::Low);
}

#[test]
fn test_task_update_with_status_transition() {
    // Setup
    let mut state = TaskBoardState::new();

    // Create task in Todo
    let mut task = create_sample_task(
        TaskStatus::Todo,
        TaskPriority::High,
        TaskComplexity::Complex,
    );
    update(&mut state, TaskBoardMessage::TaskCreated(task.clone()));

    // Verify initial state
    assert_eq!(state.kanban_board.todo.len(), 1);
    assert_eq!(state.kanban_board.in_progress.len(), 0);
    assert_eq!(state.kanban_board.done.len(), 0);

    // Transition: Todo -> InProgress
    task.status = TaskStatus::InProgress;
    update(&mut state, TaskBoardMessage::TaskUpdated(task.clone()));

    assert_eq!(state.kanban_board.todo.len(), 0);
    assert_eq!(state.kanban_board.in_progress.len(), 1);
    assert_eq!(state.kanban_board.done.len(), 0);

    // Transition: InProgress -> Done
    task.status = TaskStatus::Done;
    update(&mut state, TaskBoardMessage::TaskUpdated(task.clone()));

    assert_eq!(state.kanban_board.todo.len(), 0);
    assert_eq!(state.kanban_board.in_progress.len(), 0);
    assert_eq!(state.kanban_board.done.len(), 1);
}

#[test]
fn test_connection_status_handling() {
    // Setup
    let mut state = TaskBoardState::new();
    assert!(state.realtime_state.connected == false);
    assert!(state.error.is_none());

    // Simulate connection
    update(&mut state, TaskBoardMessage::ConnectionStatusChanged(true));

    assert!(state.realtime_state.connected);
    assert!(state.error.is_none());

    // Simulate disconnection
    update(&mut state, TaskBoardMessage::ConnectionStatusChanged(false));

    assert!(!state.realtime_state.connected);
    assert!(state.error.is_some());
    assert_eq!(state.error.as_ref().unwrap(), "Real-time connection lost");
}

#[test]
fn test_realtime_toggle() {
    // Setup
    let mut state = TaskBoardState::new();
    assert!(state.realtime_state.enabled);

    // Disable real-time updates
    update(&mut state, TaskBoardMessage::ToggleRealtimeUpdates);
    assert!(!state.realtime_state.enabled);

    // Try to add a task (should not be applied)
    let task = create_sample_task(TaskStatus::Todo, TaskPriority::High, TaskComplexity::Simple);
    update(&mut state, TaskBoardMessage::TaskCreated(task.clone()));

    // Task should not be added when real-time is disabled
    assert_eq!(state.kanban_board.todo.len(), 0);

    // Re-enable real-time updates
    update(&mut state, TaskBoardMessage::ToggleRealtimeUpdates);
    assert!(state.realtime_state.enabled);

    // Now adding should work
    update(&mut state, TaskBoardMessage::TaskCreated(task.clone()));
    assert_eq!(state.kanban_board.todo.len(), 1);
}

#[test]
fn test_debouncing_state() {
    // Setup
    let mut state = TaskBoardState::new();
    let task = create_sample_task(TaskStatus::Todo, TaskPriority::High, TaskComplexity::Simple);

    // First update should be applied immediately
    assert!(state.should_apply_update(&task.id));

    // Mark as pending
    state.mark_pending_update(task.id);

    // Immediate subsequent update should be debounced
    assert!(!state.should_apply_update(&task.id));

    // Clear pending
    state.clear_pending_update(&task.id);

    // Should now allow update
    assert!(state.should_apply_update(&task.id));
}

#[test]
fn test_selected_task_deselection_on_delete() {
    // Setup
    let mut state = TaskBoardState::new();

    // Create and select a task
    let task = create_sample_task(
        TaskStatus::Done,
        TaskPriority::Medium,
        TaskComplexity::Moderate,
    );
    update(&mut state, TaskBoardMessage::TaskCreated(task.clone()));
    update(&mut state, TaskBoardMessage::TaskClicked(task.id));

    assert_eq!(state.selected_task, Some(task.id));

    // Delete the selected task
    update(&mut state, TaskBoardMessage::TaskDeleted(task.id));

    // Verify task is removed and deselected
    assert_eq!(state.kanban_board.done.len(), 0);
    assert_eq!(state.selected_task, None);
}

#[test]
fn test_task_event_processing() {
    // Setup
    let mut state = TaskBoardState::new();

    // Test Created event processing
    let task = create_sample_task(TaskStatus::Todo, TaskPriority::High, TaskComplexity::Simple);
    let created_event = create_task_event(&task, TaskEventType::Created, true);

    if let Some(msg) = state.process_event(created_event.clone()) {
        update(&mut state, msg);
    }

    assert_eq!(state.kanban_board.todo.len(), 1);
    assert_eq!(state.realtime_state.events_received, 1);

    // Test Updated event processing
    let mut updated_task = task.clone();
    updated_task.status = TaskStatus::InProgress;
    let update_event = create_task_event(&updated_task, TaskEventType::Progress, true);

    if let Some(msg) = state.process_event(update_event) {
        update(&mut state, msg);
    }

    assert_eq!(state.kanban_board.todo.len(), 0);
    assert_eq!(state.kanban_board.in_progress.len(), 1);
    assert_eq!(state.realtime_state.events_received, 2);

    // Test Deleted event processing
    let delete_event = create_task_event(&updated_task, TaskEventType::Cancelled, false);

    if let Some(msg) = state.process_event(delete_event) {
        update(&mut state, msg);
    }

    assert_eq!(state.kanban_board.in_progress.len(), 0);
    assert_eq!(state.realtime_state.events_received, 3);
}

#[test]
fn test_multiple_column_transitions() {
    // Setup
    let mut state = TaskBoardState::new();

    // Create task
    let mut task = create_sample_task(
        TaskStatus::Todo,
        TaskPriority::Critical,
        TaskComplexity::Epic,
    );
    update(&mut state, TaskBoardMessage::TaskCreated(task.clone()));

    // Track statistics
    let initial_updates = state.realtime_state.updates_applied;

    // Transition through all states
    let transitions = vec![
        TaskStatus::InProgress,
        TaskStatus::Blocked,
        TaskStatus::InProgress,
        TaskStatus::Done,
    ];

    for new_status in transitions {
        task.status = new_status;
        task.updated_at = Utc::now().timestamp();
        update(&mut state, TaskBoardMessage::TaskUpdated(task.clone()));
    }

    // Verify final state
    assert_eq!(state.kanban_board.done.len(), 1);
    assert_eq!(state.kanban_board.todo.len(), 0);
    assert_eq!(state.kanban_board.in_progress.len(), 0);
    assert_eq!(state.kanban_board.blocked.len(), 0);

    // Verify statistics
    assert_eq!(
        state.realtime_state.updates_applied,
        initial_updates + 5 // 1 create + 4 updates
    );
}

#[test]
fn test_flush_pending_updates() {
    // Setup
    let mut state = TaskBoardState::new();

    // Add some pending updates
    let task1_id = Uuid::new_v4();
    let task2_id = Uuid::new_v4();
    let task3_id = Uuid::new_v4();

    state.mark_pending_update(task1_id);
    state.mark_pending_update(task2_id);
    state.mark_pending_update(task3_id);

    assert_eq!(state.realtime_state.pending_updates.len(), 3);

    // Flush pending updates
    update(&mut state, TaskBoardMessage::FlushPendingUpdates);

    // Verify all pending updates are cleared
    assert_eq!(state.realtime_state.pending_updates.len(), 0);
}

#[test]
fn test_event_statistics() {
    // Setup
    let mut state = TaskBoardState::new();

    assert_eq!(state.realtime_state.events_received, 0);
    assert_eq!(state.realtime_state.updates_applied, 0);

    // Process some events
    for i in 0..5 {
        let task = create_sample_task(
            TaskStatus::Todo,
            TaskPriority::Medium,
            TaskComplexity::Moderate,
        );
        let event = create_task_event(&task, TaskEventType::Created, true);
        update(&mut state, TaskBoardMessage::EventReceived(event));
    }

    // Verify statistics
    assert_eq!(state.realtime_state.events_received, 5);
    assert_eq!(state.realtime_state.updates_applied, 5);
}

#[test]
fn test_complex_filter_and_update_scenario() {
    // Setup
    let mut state = TaskBoardState::new();

    // Set up complex filter
    update(
        &mut state,
        TaskBoardMessage::FilterByPriority(Some(TaskPriority::High)),
    );
    update(
        &mut state,
        TaskBoardMessage::FilterByComplexity(Some(TaskComplexity::Complex)),
    );

    // Add tasks with various attributes
    let matching_task = create_sample_task(
        TaskStatus::Todo,
        TaskPriority::High,
        TaskComplexity::Complex,
    );
    let non_matching_task1 =
        create_sample_task(TaskStatus::Todo, TaskPriority::Low, TaskComplexity::Complex);
    let non_matching_task2 =
        create_sample_task(TaskStatus::Todo, TaskPriority::High, TaskComplexity::Simple);

    update(
        &mut state,
        TaskBoardMessage::TaskCreated(matching_task.clone()),
    );
    update(
        &mut state,
        TaskBoardMessage::TaskCreated(non_matching_task1.clone()),
    );
    update(
        &mut state,
        TaskBoardMessage::TaskCreated(non_matching_task2.clone()),
    );

    // All tasks should be stored
    assert_eq!(state.kanban_board.todo.len(), 3);

    // Apply filters
    let filtered = state.apply_filters(state.kanban_board.todo.clone());

    // Only matching task should be returned
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, matching_task.id);

    // Verify filters persist after updates
    assert_eq!(state.filters.priority, Some(TaskPriority::High));
    assert_eq!(state.filters.complexity, Some(TaskComplexity::Complex));
}
