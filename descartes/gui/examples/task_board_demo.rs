/// Task Board Demo Application
///
/// This example demonstrates the Task Board GUI component with sample data.
/// It shows how to:
/// - Initialize the task board state
/// - Load sample tasks
/// - Display tasks in a Kanban layout
/// - Filter and sort tasks
/// - Handle task interactions

use descartes_core::{Task, TaskComplexity, TaskPriority, TaskStatus};
use descartes_gui::task_board::{KanbanBoard, TaskBoardMessage, TaskBoardState};
use iced::widget::{button, column, container, text, Space};
use iced::{Element, Length, Size, Theme, window};
use uuid::Uuid;

fn main() -> iced::Result {
    iced::application("Task Board Demo", TaskBoardDemo::update, TaskBoardDemo::view)
        .window(window::Settings {
            size: Size::new(1400.0, 900.0),
            position: window::Position::Centered,
            min_size: Some(Size::new(1000.0, 700.0)),
            ..Default::default()
        })
        .theme(|_| Theme::TokyoNight)
        .run_with(|| (TaskBoardDemo::new(), iced::Task::none()))
}

struct TaskBoardDemo {
    task_board_state: TaskBoardState,
}

#[derive(Debug, Clone)]
enum Message {
    TaskBoard(TaskBoardMessage),
    LoadSampleTasks,
}

impl TaskBoardDemo {
    fn new() -> Self {
        Self {
            task_board_state: TaskBoardState::default(),
        }
    }

    fn update(&mut self, message: Message) -> iced::Task<Message> {
        match message {
            Message::TaskBoard(msg) => {
                descartes_gui::task_board::update(&mut self.task_board_state, msg);
            }
            Message::LoadSampleTasks => {
                self.load_sample_tasks();
            }
        }
        iced::Task::none()
    }

    fn view(&self) -> Element<Message> {
        let total_tasks = self.task_board_state.kanban_board.todo.len()
            + self.task_board_state.kanban_board.in_progress.len()
            + self.task_board_state.kanban_board.done.len()
            + self.task_board_state.kanban_board.blocked.len();

        if total_tasks == 0 {
            let welcome = column![
                text("Task Board Demo").size(32),
                Space::with_height(20),
                text("Welcome to the Descartes Task Board!").size(18),
                Space::with_height(10),
                text("This is a demonstration of the Kanban-style task board component.").size(14),
                Space::with_height(20),
                button(text("Load Sample Tasks").size(16))
                    .on_press(Message::LoadSampleTasks)
                    .padding(15),
            ]
            .spacing(10)
            .padding(40);

            container(welcome)
                .center_x(Length::Fill)
                .center_y(Length::Fill)
                .into()
        } else {
            descartes_gui::task_board::view(&self.task_board_state).map(Message::TaskBoard)
        }
    }

    fn load_sample_tasks(&mut self) {
        use chrono::Utc;
        let now = Utc::now().timestamp();

        // Create comprehensive sample tasks
        let mut todo_tasks = vec![];
        let mut in_progress_tasks = vec![];
        let mut done_tasks = vec![];
        let mut blocked_tasks = vec![];

        // === TODO TASKS ===

        // Critical priority tasks
        todo_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Fix critical security vulnerability".to_string(),
            description: Some("SQL injection vulnerability in user input validation. Needs immediate attention.".to_string()),
            status: TaskStatus::Todo,
            priority: TaskPriority::Critical,
            complexity: TaskComplexity::Complex,
            assigned_to: Some("security-team".to_string()),
            dependencies: vec![],
            created_at: now - 3600,
            updated_at: now - 3600,
            metadata: None,
        });

        // High priority tasks
        todo_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Implement user authentication".to_string(),
            description: Some("Add JWT-based authentication with refresh tokens and proper session management.".to_string()),
            status: TaskStatus::Todo,
            priority: TaskPriority::High,
            complexity: TaskComplexity::Complex,
            assigned_to: Some("alice".to_string()),
            dependencies: vec![],
            created_at: now - 86400,
            updated_at: now - 7200,
            metadata: None,
        });

        todo_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Optimize database queries".to_string(),
            description: Some("Add indexes and optimize slow queries identified in production monitoring.".to_string()),
            status: TaskStatus::Todo,
            priority: TaskPriority::High,
            complexity: TaskComplexity::Moderate,
            assigned_to: Some("bob".to_string()),
            dependencies: vec![],
            created_at: now - 43200,
            updated_at: now - 43200,
            metadata: None,
        });

        // Medium priority tasks
        todo_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Write unit tests for parser".to_string(),
            description: Some("Achieve 90% code coverage for the expression parser module.".to_string()),
            status: TaskStatus::Todo,
            priority: TaskPriority::Medium,
            complexity: TaskComplexity::Moderate,
            assigned_to: Some("charlie".to_string()),
            dependencies: vec![],
            created_at: now - 28800,
            updated_at: now - 28800,
            metadata: None,
        });

        todo_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Update API documentation".to_string(),
            description: Some("Refresh OpenAPI specs with new endpoints and deprecation notices.".to_string()),
            status: TaskStatus::Todo,
            priority: TaskPriority::Medium,
            complexity: TaskComplexity::Simple,
            assigned_to: None,
            dependencies: vec![],
            created_at: now - 14400,
            updated_at: now - 14400,
            metadata: None,
        });

        // Low priority tasks
        todo_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Refactor legacy code".to_string(),
            description: Some("Clean up old code patterns and improve maintainability.".to_string()),
            status: TaskStatus::Todo,
            priority: TaskPriority::Low,
            complexity: TaskComplexity::Epic,
            assigned_to: None,
            dependencies: vec![],
            created_at: now - 172800,
            updated_at: now - 172800,
            metadata: None,
        });

        // === IN PROGRESS TASKS ===

        in_progress_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Migrate to microservices architecture".to_string(),
            description: Some("Break down monolith into independent services. Currently extracting user service.".to_string()),
            status: TaskStatus::InProgress,
            priority: TaskPriority::High,
            complexity: TaskComplexity::Epic,
            assigned_to: Some("alice".to_string()),
            dependencies: vec![],
            created_at: now - 604800,
            updated_at: now - 1800,
            metadata: None,
        });

        in_progress_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Implement real-time notifications".to_string(),
            description: Some("Add WebSocket support for push notifications to users.".to_string()),
            status: TaskStatus::InProgress,
            priority: TaskPriority::High,
            complexity: TaskComplexity::Complex,
            assigned_to: Some("bob".to_string()),
            dependencies: vec![],
            created_at: now - 259200,
            updated_at: now - 900,
            metadata: None,
        });

        in_progress_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Design new UI mockups".to_string(),
            description: Some("Create Figma designs for dashboard redesign with user feedback incorporated.".to_string()),
            status: TaskStatus::InProgress,
            priority: TaskPriority::Medium,
            complexity: TaskComplexity::Moderate,
            assigned_to: Some("design-team".to_string()),
            dependencies: vec![],
            created_at: now - 86400,
            updated_at: now - 600,
            metadata: None,
        });

        in_progress_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Set up monitoring dashboard".to_string(),
            description: Some("Configure Grafana dashboards for application metrics.".to_string()),
            status: TaskStatus::InProgress,
            priority: TaskPriority::Medium,
            complexity: TaskComplexity::Simple,
            assigned_to: Some("ops-team".to_string()),
            dependencies: vec![],
            created_at: now - 43200,
            updated_at: now - 300,
            metadata: None,
        });

        // === DONE TASKS ===

        done_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Setup CI/CD pipeline".to_string(),
            description: Some("Configured GitHub Actions for automated testing and deployment to staging.".to_string()),
            status: TaskStatus::Done,
            priority: TaskPriority::High,
            complexity: TaskComplexity::Complex,
            assigned_to: Some("bob".to_string()),
            dependencies: vec![],
            created_at: now - 432000,
            updated_at: now - 86400,
            metadata: None,
        });

        done_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Fix login timeout bug".to_string(),
            description: Some("Resolved issue where sessions were expiring prematurely.".to_string()),
            status: TaskStatus::Done,
            priority: TaskPriority::Critical,
            complexity: TaskComplexity::Simple,
            assigned_to: Some("alice".to_string()),
            dependencies: vec![],
            created_at: now - 259200,
            updated_at: now - 172800,
            metadata: None,
        });

        done_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Upgrade dependencies".to_string(),
            description: Some("Updated all npm packages to latest stable versions.".to_string()),
            status: TaskStatus::Done,
            priority: TaskPriority::Medium,
            complexity: TaskComplexity::Moderate,
            assigned_to: Some("charlie".to_string()),
            dependencies: vec![],
            created_at: now - 345600,
            updated_at: now - 259200,
            metadata: None,
        });

        done_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Write deployment guide".to_string(),
            description: Some("Comprehensive documentation for production deployment procedures.".to_string()),
            status: TaskStatus::Done,
            priority: TaskPriority::Low,
            complexity: TaskComplexity::Simple,
            assigned_to: None,
            dependencies: vec![],
            created_at: now - 518400,
            updated_at: now - 432000,
            metadata: None,
        });

        // === BLOCKED TASKS ===

        let dep_task_id = Uuid::new_v4();
        blocked_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Deploy to production".to_string(),
            description: Some("Waiting for security audit completion before production deployment.".to_string()),
            status: TaskStatus::Blocked,
            priority: TaskPriority::Critical,
            complexity: TaskComplexity::Moderate,
            assigned_to: Some("ops-team".to_string()),
            dependencies: vec![dep_task_id],
            created_at: now - 86400,
            updated_at: now - 7200,
            metadata: None,
        });

        blocked_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Integrate payment gateway".to_string(),
            description: Some("Blocked pending legal approval for payment processor contract.".to_string()),
            status: TaskStatus::Blocked,
            priority: TaskPriority::High,
            complexity: TaskComplexity::Complex,
            assigned_to: Some("alice".to_string()),
            dependencies: vec![dep_task_id],
            created_at: now - 172800,
            updated_at: now - 43200,
            metadata: None,
        });

        blocked_tasks.push(Task {
            id: Uuid::new_v4(),
            title: "Optimize image loading".to_string(),
            description: Some("Blocked on infrastructure team to set up CDN and configure caching.".to_string()),
            status: TaskStatus::Blocked,
            priority: TaskPriority::Low,
            complexity: TaskComplexity::Simple,
            assigned_to: Some("frontend-team".to_string()),
            dependencies: vec![dep_task_id],
            created_at: now - 259200,
            updated_at: now - 86400,
            metadata: None,
        });

        // Create and load the kanban board
        let board = KanbanBoard {
            todo: todo_tasks,
            in_progress: in_progress_tasks,
            done: done_tasks,
            blocked: blocked_tasks,
        };

        descartes_gui::task_board::update(
            &mut self.task_board_state,
            TaskBoardMessage::TasksLoaded(board),
        );
    }
}
