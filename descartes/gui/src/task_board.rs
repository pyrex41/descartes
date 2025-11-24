/// Task Board GUI Component
///
/// Provides a Kanban-style task board with:
/// - Multiple columns for different task statuses (Todo, InProgress, Done, Blocked)
/// - Task cards with priority, complexity, and dependency indicators
/// - Filtering and sorting controls
/// - Real-time updates from daemon

use descartes_core::{Task, TaskComplexity, TaskPriority, TaskStatus};
use iced::widget::{button, column, container, row, scrollable, text, Column, Row, Space};
use iced::{alignment, color, Alignment, Color, Element, Length, Theme};
use uuid::Uuid;

/// State for the Task Board view
#[derive(Debug, Clone)]
pub struct TaskBoardState {
    /// All tasks organized by status
    pub kanban_board: KanbanBoard,
    /// Current filter settings
    pub filters: TaskFilters,
    /// Current sort settings
    pub sort: TaskSort,
    /// Selected task (for details view)
    pub selected_task: Option<Uuid>,
    /// Loading state
    pub loading: bool,
    /// Error message
    pub error: Option<String>,
}

/// Kanban board structure
#[derive(Debug, Clone, Default)]
pub struct KanbanBoard {
    pub todo: Vec<Task>,
    pub in_progress: Vec<Task>,
    pub done: Vec<Task>,
    pub blocked: Vec<Task>,
}

/// Task filter settings
#[derive(Debug, Clone)]
pub struct TaskFilters {
    pub priority: Option<TaskPriority>,
    pub complexity: Option<TaskComplexity>,
    pub assignee: Option<String>,
    pub search_term: Option<String>,
    pub show_blocked_only: bool,
}

impl Default for TaskFilters {
    fn default() -> Self {
        Self {
            priority: None,
            complexity: None,
            assignee: None,
            search_term: None,
            show_blocked_only: false,
        }
    }
}

/// Task sorting settings
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskSort {
    Priority,
    Complexity,
    CreatedAt,
    UpdatedAt,
    Title,
}

impl Default for TaskSort {
    fn default() -> Self {
        Self::Priority
    }
}

/// Messages for task board interactions
#[derive(Debug, Clone)]
pub enum TaskBoardMessage {
    /// Task was clicked
    TaskClicked(Uuid),
    /// Load tasks from daemon
    LoadTasks,
    /// Tasks loaded successfully
    TasksLoaded(KanbanBoard),
    /// Error loading tasks
    LoadError(String),
    /// Filter by priority
    FilterByPriority(Option<TaskPriority>),
    /// Filter by complexity
    FilterByComplexity(Option<TaskComplexity>),
    /// Filter by assignee
    FilterByAssignee(Option<String>),
    /// Search tasks
    SearchTasks(String),
    /// Clear all filters
    ClearFilters,
    /// Change sort order
    ChangeSortOrder(TaskSort),
    /// Refresh tasks
    RefreshTasks,
    /// Toggle show blocked only
    ToggleBlockedOnly,
}

impl Default for TaskBoardState {
    fn default() -> Self {
        Self {
            kanban_board: KanbanBoard::default(),
            filters: TaskFilters::default(),
            sort: TaskSort::default(),
            selected_task: None,
            loading: false,
            error: None,
        }
    }
}

impl TaskBoardState {
    /// Create a new task board state
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply filters to tasks
    pub fn apply_filters(&self, tasks: Vec<Task>) -> Vec<Task> {
        tasks
            .into_iter()
            .filter(|task| {
                // Priority filter
                if let Some(priority) = self.filters.priority {
                    if task.priority != priority {
                        return false;
                    }
                }

                // Complexity filter
                if let Some(complexity) = self.filters.complexity {
                    if task.complexity != complexity {
                        return false;
                    }
                }

                // Assignee filter
                if let Some(ref assignee) = self.filters.assignee {
                    if task.assigned_to.as_ref() != Some(assignee) {
                        return false;
                    }
                }

                // Search term filter
                if let Some(ref term) = self.filters.search_term {
                    let term_lower = term.to_lowercase();
                    let title_match = task.title.to_lowercase().contains(&term_lower);
                    let desc_match = task
                        .description
                        .as_ref()
                        .map(|d| d.to_lowercase().contains(&term_lower))
                        .unwrap_or(false);
                    if !title_match && !desc_match {
                        return false;
                    }
                }

                // Blocked only filter
                if self.filters.show_blocked_only && task.status != TaskStatus::Blocked {
                    return false;
                }

                true
            })
            .collect()
    }

    /// Sort tasks
    pub fn sort_tasks(&self, mut tasks: Vec<Task>) -> Vec<Task> {
        match self.sort {
            TaskSort::Priority => {
                tasks.sort_by(|a, b| b.priority.cmp(&a.priority));
            }
            TaskSort::Complexity => {
                tasks.sort_by(|a, b| b.complexity.cmp(&a.complexity));
            }
            TaskSort::CreatedAt => {
                tasks.sort_by(|a, b| b.created_at.cmp(&a.created_at));
            }
            TaskSort::UpdatedAt => {
                tasks.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
            }
            TaskSort::Title => {
                tasks.sort_by(|a, b| a.title.cmp(&b.title));
            }
        }
        tasks
    }
}

/// Render the task board view
pub fn view(state: &TaskBoardState) -> Element<TaskBoardMessage> {
    let header = view_header(state);
    let filters = view_filters(state);
    let board = view_kanban_board(state);

    let content = column![header, filters, board]
        .spacing(20)
        .padding(10)
        .width(Length::Fill)
        .height(Length::Fill);

    container(content)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Render the header with title and stats
fn view_header(state: &TaskBoardState) -> Element<TaskBoardMessage> {
    let title = text("Task Board").size(32);

    let total_count = state.kanban_board.todo.len()
        + state.kanban_board.in_progress.len()
        + state.kanban_board.done.len()
        + state.kanban_board.blocked.len();

    let stats = text(format!(
        "Total: {} | Todo: {} | In Progress: {} | Done: {} | Blocked: {}",
        total_count,
        state.kanban_board.todo.len(),
        state.kanban_board.in_progress.len(),
        state.kanban_board.done.len(),
        state.kanban_board.blocked.len(),
    ))
    .size(14);

    let refresh_btn = button(text("Refresh").size(14))
        .on_press(TaskBoardMessage::RefreshTasks)
        .padding(8);

    let header_row = row![title, Space::with_width(Length::Fill), stats, refresh_btn]
        .spacing(10)
        .align_y(Alignment::Center);

    container(header_row)
        .width(Length::Fill)
        .padding(10)
        .into()
}

/// Render the filter controls
fn view_filters(state: &TaskBoardState) -> Element<TaskBoardMessage> {
    let priority_filter = view_priority_filter(state);
    let complexity_filter = view_complexity_filter(state);
    let sort_control = view_sort_control(state);

    let blocked_toggle = button(text(if state.filters.show_blocked_only {
        "Show All"
    } else {
        "Blocked Only"
    }))
    .on_press(TaskBoardMessage::ToggleBlockedOnly)
    .padding(8);

    let clear_filters = button(text("Clear Filters"))
        .on_press(TaskBoardMessage::ClearFilters)
        .padding(8);

    let filters_row = row![
        text("Filters:").size(16),
        priority_filter,
        complexity_filter,
        text("Sort:").size(16),
        sort_control,
        blocked_toggle,
        clear_filters,
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    container(filters_row)
        .width(Length::Fill)
        .padding(10)
        .style(|theme: &Theme| container::Style {
            background: Some(color!(0x2a2a2a).into()),
            border: iced::Border {
                width: 1.0,
                color: color!(0x404040),
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into()
}

/// Render priority filter dropdown
fn view_priority_filter(state: &TaskBoardState) -> Element<TaskBoardMessage> {
    let current = state
        .filters
        .priority
        .map(|p| format!("{:?}", p))
        .unwrap_or_else(|| "All Priorities".to_string());

    let btn = button(text(current).size(14)).padding(8);

    // In a real implementation, this would be a dropdown menu
    // For now, we'll use a simple button that cycles through options
    container(btn).into()
}

/// Render complexity filter dropdown
fn view_complexity_filter(state: &TaskBoardState) -> Element<TaskBoardMessage> {
    let current = state
        .filters
        .complexity
        .map(|c| format!("{:?}", c))
        .unwrap_or_else(|| "All Complexities".to_string());

    let btn = button(text(current).size(14)).padding(8);

    container(btn).into()
}

/// Render sort control dropdown
fn view_sort_control(state: &TaskBoardState) -> Element<TaskBoardMessage> {
    let current = match state.sort {
        TaskSort::Priority => "Priority",
        TaskSort::Complexity => "Complexity",
        TaskSort::CreatedAt => "Created",
        TaskSort::UpdatedAt => "Updated",
        TaskSort::Title => "Title",
    };

    let btn = button(text(current).size(14)).padding(8);

    container(btn).into()
}

/// Render the Kanban board with columns
fn view_kanban_board(state: &TaskBoardState) -> Element<TaskBoardMessage> {
    let todo_col = view_kanban_column(
        "Todo",
        &state.kanban_board.todo,
        state,
        color!(0x3498db), // Blue
    );
    let in_progress_col = view_kanban_column(
        "In Progress",
        &state.kanban_board.in_progress,
        state,
        color!(0xf39c12), // Orange
    );
    let done_col = view_kanban_column(
        "Done",
        &state.kanban_board.done,
        state,
        color!(0x2ecc71), // Green
    );
    let blocked_col = view_kanban_column(
        "Blocked",
        &state.kanban_board.blocked,
        state,
        color!(0xe74c3c), // Red
    );

    let board_row = row![todo_col, in_progress_col, done_col, blocked_col]
        .spacing(10)
        .width(Length::Fill)
        .height(Length::Fill);

    container(board_row)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Render a single Kanban column
fn view_kanban_column<'a>(
    title: &str,
    tasks: &[Task],
    state: &TaskBoardState,
    color: Color,
) -> Element<'a, TaskBoardMessage> {
    let header = container(text(title).size(18))
        .width(Length::Fill)
        .padding(10)
        .style(move |theme: &Theme| container::Style {
            background: Some(color.into()),
            border: iced::Border {
                width: 0.0,
                color: Color::TRANSPARENT,
                radius: [4.0, 4.0, 0.0, 0.0].into(),
            },
            ..Default::default()
        });

    let count = text(format!("{} tasks", tasks.len()))
        .size(12)
        .color(color!(0x888888));

    // Apply filters and sorting
    let filtered_tasks = state.apply_filters(tasks.to_vec());
    let sorted_tasks = state.sort_tasks(filtered_tasks);

    // Create task cards
    let mut card_column = Column::new().spacing(8).padding(10);

    for task in sorted_tasks.iter() {
        card_column = card_column.push(view_task_card(task, state));
    }

    let scrollable_content = scrollable(card_column)
        .height(Length::Fill)
        .width(Length::Fill);

    let column_content = column![header, count, scrollable_content]
        .spacing(5)
        .width(Length::Fill)
        .height(Length::Fill);

    container(column_content)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|theme: &Theme| container::Style {
            background: Some(color!(0x1e1e1e).into()),
            border: iced::Border {
                width: 1.0,
                color: color!(0x404040),
                radius: 4.0.into(),
            },
            ..Default::default()
        })
        .into()
}

/// Render a task card
fn view_task_card<'a>(task: &Task, state: &TaskBoardState) -> Element<'a, TaskBoardMessage> {
    let is_selected = state.selected_task.as_ref() == Some(&task.id);

    // Title
    let title = text(&task.title)
        .size(14)
        .color(if is_selected {
            color!(0xffffff)
        } else {
            color!(0xdddddd)
        });

    // Priority badge
    let priority_badge = view_priority_badge(task.priority);

    // Complexity badge
    let complexity_badge = view_complexity_badge(task.complexity);

    // Assignee info
    let assignee_text = if let Some(ref assignee) = task.assigned_to {
        text(format!("@{}", assignee)).size(12).color(color!(0x888888))
    } else {
        text("Unassigned").size(12).color(color!(0x666666))
    };

    // Dependency indicator
    let dependency_indicator = if !task.dependencies.is_empty() {
        Some(
            container(text(format!("{} deps", task.dependencies.len())))
                .padding(4)
                .style(|theme: &Theme| container::Style {
                    background: Some(color!(0x9b59b6).into()),
                    border: iced::Border {
                        width: 0.0,
                        color: Color::TRANSPARENT,
                        radius: 3.0.into(),
                    },
                    ..Default::default()
                }),
        )
    } else {
        None
    };

    // Build the card layout
    let mut card_content = Column::new().spacing(8).padding(10);

    // Title row
    card_content = card_content.push(title);

    // Badges row
    let mut badges_row = Row::new().spacing(5);
    badges_row = badges_row.push(priority_badge);
    badges_row = badges_row.push(complexity_badge);
    if let Some(dep_indicator) = dependency_indicator {
        badges_row = badges_row.push(dep_indicator);
    }
    card_content = card_content.push(badges_row);

    // Assignee row
    card_content = card_content.push(assignee_text);

    // Description (truncated)
    if let Some(ref desc) = task.description {
        let truncated = if desc.len() > 60 {
            format!("{}...", &desc[..60])
        } else {
            desc.clone()
        };
        card_content = card_content.push(text(truncated).size(12).color(color!(0x999999)));
    }

    let task_id = task.id;
    let card_button = button(card_content)
        .on_press(TaskBoardMessage::TaskClicked(task_id))
        .width(Length::Fill)
        .style(move |theme: &Theme, status| {
            let base_color = if is_selected {
                color!(0x3a3a3a)
            } else {
                color!(0x2a2a2a)
            };

            let border_color = if is_selected {
                color!(0x3498db)
            } else {
                color!(0x404040)
            };

            button::Style {
                background: Some(base_color.into()),
                text_color: color!(0xffffff),
                border: iced::Border {
                    width: if is_selected { 2.0 } else { 1.0 },
                    color: border_color,
                    radius: 4.0.into(),
                },
                ..button::Style::default()
            }
        });

    container(card_button)
        .width(Length::Fill)
        .into()
}

/// Render a priority badge
fn view_priority_badge(priority: TaskPriority) -> Element<'static, TaskBoardMessage> {
    let (label, badge_color) = match priority {
        TaskPriority::Critical => ("CRITICAL", color!(0xe74c3c)),
        TaskPriority::High => ("HIGH", color!(0xf39c12)),
        TaskPriority::Medium => ("MED", color!(0x3498db)),
        TaskPriority::Low => ("LOW", color!(0x95a5a6)),
    };

    container(text(label).size(10).color(color!(0xffffff)))
        .padding(4)
        .style(move |theme: &Theme| container::Style {
            background: Some(badge_color.into()),
            border: iced::Border {
                width: 0.0,
                color: Color::TRANSPARENT,
                radius: 3.0.into(),
            },
            ..Default::default()
        })
        .into()
}

/// Render a complexity badge
fn view_complexity_badge(complexity: TaskComplexity) -> Element<'static, TaskBoardMessage> {
    let (label, badge_color) = match complexity {
        TaskComplexity::Trivial => ("TRIVIAL", color!(0x95a5a6)),
        TaskComplexity::Simple => ("SIMPLE", color!(0x3498db)),
        TaskComplexity::Moderate => ("MODERATE", color!(0xf39c12)),
        TaskComplexity::Complex => ("COMPLEX", color!(0xe67e22)),
        TaskComplexity::Epic => ("EPIC", color!(0xe74c3c)),
    };

    container(text(label).size(10).color(color!(0xffffff)))
        .padding(4)
        .style(move |theme: &Theme| container::Style {
            background: Some(badge_color.into()),
            border: iced::Border {
                width: 0.0,
                color: Color::TRANSPARENT,
                radius: 3.0.into(),
            },
            ..Default::default()
        })
        .into()
}

/// Update the task board state based on messages
pub fn update(state: &mut TaskBoardState, message: TaskBoardMessage) {
    match message {
        TaskBoardMessage::TaskClicked(task_id) => {
            state.selected_task = Some(task_id);
        }
        TaskBoardMessage::LoadTasks => {
            state.loading = true;
            state.error = None;
        }
        TaskBoardMessage::TasksLoaded(board) => {
            state.kanban_board = board;
            state.loading = false;
        }
        TaskBoardMessage::LoadError(err) => {
            state.error = Some(err);
            state.loading = false;
        }
        TaskBoardMessage::FilterByPriority(priority) => {
            state.filters.priority = priority;
        }
        TaskBoardMessage::FilterByComplexity(complexity) => {
            state.filters.complexity = complexity;
        }
        TaskBoardMessage::FilterByAssignee(assignee) => {
            state.filters.assignee = assignee;
        }
        TaskBoardMessage::SearchTasks(term) => {
            state.filters.search_term = if term.is_empty() { None } else { Some(term) };
        }
        TaskBoardMessage::ClearFilters => {
            state.filters = TaskFilters::default();
        }
        TaskBoardMessage::ChangeSortOrder(sort) => {
            state.sort = sort;
        }
        TaskBoardMessage::RefreshTasks => {
            state.loading = true;
            state.error = None;
        }
        TaskBoardMessage::ToggleBlockedOnly => {
            state.filters.show_blocked_only = !state.filters.show_blocked_only;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    #[test]
    fn test_task_board_state_creation() {
        let state = TaskBoardState::new();
        assert_eq!(state.kanban_board.todo.len(), 0);
        assert_eq!(state.kanban_board.in_progress.len(), 0);
        assert_eq!(state.kanban_board.done.len(), 0);
        assert_eq!(state.kanban_board.blocked.len(), 0);
    }

    #[test]
    fn test_apply_filters_priority() {
        let state = TaskBoardState {
            filters: TaskFilters {
                priority: Some(TaskPriority::High),
                ..Default::default()
            },
            ..Default::default()
        };

        let tasks = vec![
            Task {
                id: Uuid::new_v4(),
                title: "High priority task".to_string(),
                description: None,
                status: TaskStatus::Todo,
                priority: TaskPriority::High,
                complexity: TaskComplexity::Simple,
                assigned_to: None,
                dependencies: vec![],
                created_at: Utc::now().timestamp(),
                updated_at: Utc::now().timestamp(),
                metadata: None,
            },
            Task {
                id: Uuid::new_v4(),
                title: "Low priority task".to_string(),
                description: None,
                status: TaskStatus::Todo,
                priority: TaskPriority::Low,
                complexity: TaskComplexity::Simple,
                assigned_to: None,
                dependencies: vec![],
                created_at: Utc::now().timestamp(),
                updated_at: Utc::now().timestamp(),
                metadata: None,
            },
        ];

        let filtered = state.apply_filters(tasks);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].priority, TaskPriority::High);
    }

    #[test]
    fn test_apply_filters_search() {
        let state = TaskBoardState {
            filters: TaskFilters {
                search_term: Some("authentication".to_string()),
                ..Default::default()
            },
            ..Default::default()
        };

        let tasks = vec![
            Task {
                id: Uuid::new_v4(),
                title: "Implement authentication".to_string(),
                description: None,
                status: TaskStatus::Todo,
                priority: TaskPriority::High,
                complexity: TaskComplexity::Complex,
                assigned_to: None,
                dependencies: vec![],
                created_at: Utc::now().timestamp(),
                updated_at: Utc::now().timestamp(),
                metadata: None,
            },
            Task {
                id: Uuid::new_v4(),
                title: "Fix bug in parser".to_string(),
                description: None,
                status: TaskStatus::Todo,
                priority: TaskPriority::Medium,
                complexity: TaskComplexity::Simple,
                assigned_to: None,
                dependencies: vec![],
                created_at: Utc::now().timestamp(),
                updated_at: Utc::now().timestamp(),
                metadata: None,
            },
        ];

        let filtered = state.apply_filters(tasks);
        assert_eq!(filtered.len(), 1);
        assert!(filtered[0].title.contains("authentication"));
    }

    #[test]
    fn test_sort_tasks_by_priority() {
        let state = TaskBoardState {
            sort: TaskSort::Priority,
            ..Default::default()
        };

        let tasks = vec![
            Task {
                id: Uuid::new_v4(),
                title: "Low".to_string(),
                description: None,
                status: TaskStatus::Todo,
                priority: TaskPriority::Low,
                complexity: TaskComplexity::Simple,
                assigned_to: None,
                dependencies: vec![],
                created_at: Utc::now().timestamp(),
                updated_at: Utc::now().timestamp(),
                metadata: None,
            },
            Task {
                id: Uuid::new_v4(),
                title: "Critical".to_string(),
                description: None,
                status: TaskStatus::Todo,
                priority: TaskPriority::Critical,
                complexity: TaskComplexity::Simple,
                assigned_to: None,
                dependencies: vec![],
                created_at: Utc::now().timestamp(),
                updated_at: Utc::now().timestamp(),
                metadata: None,
            },
            Task {
                id: Uuid::new_v4(),
                title: "Medium".to_string(),
                description: None,
                status: TaskStatus::Todo,
                priority: TaskPriority::Medium,
                complexity: TaskComplexity::Simple,
                assigned_to: None,
                dependencies: vec![],
                created_at: Utc::now().timestamp(),
                updated_at: Utc::now().timestamp(),
                metadata: None,
            },
        ];

        let sorted = state.sort_tasks(tasks);
        assert_eq!(sorted[0].priority, TaskPriority::Critical);
        assert_eq!(sorted[1].priority, TaskPriority::Medium);
        assert_eq!(sorted[2].priority, TaskPriority::Low);
    }
}
