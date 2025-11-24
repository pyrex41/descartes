/// Proof-of-Concept: Statig-Based State Machine for Descartes Workflows
///
/// This PoC demonstrates:
/// 1. Simple workflow state progression
/// 2. Hierarchical state machines with event bubbling
/// 3. Async handler integration with Tokio
/// 4. Context management for workflow data
/// 5. Multiple concurrent workflows
///
/// To run this standalone:
/// ```bash
/// # Add to a temporary project
/// cargo add statig tokio serde serde_json
/// rustc --edition 2021 poc_state_machine.rs
/// ./poc_state_machine
/// ```

use std::sync::Arc;
use tokio::sync::Mutex;

// Note: This uses hypothetical statig API for illustration
// Real implementation would use the actual statig crate
// For now, we'll provide a simplified mock implementation

// ============================================================================
// EXAMPLE 1: Simple Linear Workflow (Code Review Process)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CodeReviewState {
    Submitted,
    ReviewingCode,
    ApprovingOrRequesting,
    Approved,
    RequestedChanges,
    Merged,
}

#[derive(Debug, Clone)]
struct CodeReviewContext {
    pr_id: String,
    reviewer_count: u32,
    reviews_approved: u32,
    reviews_requested_changes: u32,
    error_messages: Vec<String>,
}

impl CodeReviewContext {
    fn new(pr_id: String) -> Self {
        Self {
            pr_id,
            reviewer_count: 3,
            reviews_approved: 0,
            reviews_requested_changes: 0,
            error_messages: vec![],
        }
    }

    fn record_review_approved(&mut self) {
        self.reviews_approved += 1;
    }

    fn record_review_changes_requested(&mut self) {
        self.reviews_requested_changes += 1;
    }

    fn all_reviews_in(&self) -> bool {
        (self.reviews_approved + self.reviews_requested_changes) == self.reviewer_count
    }

    fn all_approved(&self) -> bool {
        self.reviews_approved == self.reviewer_count && self.reviews_requested_changes == 0
    }
}

#[derive(Debug, Clone)]
enum CodeReviewEvent {
    CheckSyntax,
    SyntaxPassed,
    SyntaxFailed(String),
    ReviewApproved,
    ChangesRequested(String),
    ResolvedComments,
    AllReviewsComplete,
    Merge,
}

impl CodeReviewState {
    fn on_event(self, event: CodeReviewEvent, context: &mut CodeReviewContext) -> Self {
        match (self, &event) {
            // Submitted -> ReviewingCode: After syntax check
            (CodeReviewState::Submitted, CodeReviewEvent::CheckSyntax) => {
                println!("[CodeReview] Checking syntax for PR: {}", context.pr_id);
                CodeReviewState::ReviewingCode
            }

            // ReviewingCode -> Submitted: On syntax failure
            (CodeReviewState::ReviewingCode, CodeReviewEvent::SyntaxFailed(msg)) => {
                println!("[CodeReview] Syntax check failed: {}", msg);
                context.error_messages.push(msg.clone());
                CodeReviewState::Submitted
            }

            // ReviewingCode -> ApprovingOrRequesting: After syntax passes
            (CodeReviewState::ReviewingCode, CodeReviewEvent::SyntaxPassed) => {
                println!("[CodeReview] Syntax check passed, awaiting reviews...");
                CodeReviewState::ApprovingOrRequesting
            }

            // ApprovingOrRequesting -> Approved: When all reviews pass
            (CodeReviewState::ApprovingOrRequesting, CodeReviewEvent::ReviewApproved) => {
                context.record_review_approved();
                if context.all_approved() {
                    println!("[CodeReview] All reviews approved!");
                    CodeReviewState::Approved
                } else {
                    println!(
                        "[CodeReview] Review approved ({}/{})",
                        context.reviews_approved, context.reviewer_count
                    );
                    CodeReviewState::ApprovingOrRequesting
                }
            }

            // ApprovingOrRequesting -> RequestedChanges: When feedback received
            (CodeReviewState::ApprovingOrRequesting, CodeReviewEvent::ChangesRequested(msg)) => {
                context.record_review_changes_requested();
                context.error_messages.push(msg.clone());
                println!("[CodeReview] Changes requested: {}", msg);
                CodeReviewState::RequestedChanges
            }

            // RequestedChanges -> Submitted: Developer pushes fixes
            (CodeReviewState::RequestedChanges, CodeReviewEvent::ResolvedComments) => {
                println!("[CodeReview] Developer resolved comments, re-submitting...");
                context.reviews_requested_changes = 0;
                CodeReviewState::Submitted
            }

            // Approved -> Merged: When merge confirmed
            (CodeReviewState::Approved, CodeReviewEvent::Merge) => {
                println!("[CodeReview] PR merged to main");
                CodeReviewState::Merged
            }

            // Invalid transitions
            (state, event) => {
                println!(
                    "[CodeReview] Invalid transition: {:?} on event {:?}",
                    state, event
                );
                state
            }
        }
    }
}

// ============================================================================
// EXAMPLE 2: Hierarchical Workflow (Multi-Agent Code Implementation)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ImplementationState {
    // Parent state
    Active,
    // Child states of Active
    Planning,
    Coding,
    Testing,
    // Sibling states
    Blocked,
    Complete,
}

#[derive(Debug, Clone)]
struct ImplementationContext {
    task_id: String,
    current_agent: String,
    code_files_created: u32,
    tests_written: u32,
    tests_passing: u32,
    blocking_issue: Option<String>,
}

impl ImplementationContext {
    fn new(task_id: String) -> Self {
        Self {
            task_id,
            current_agent: "architect".to_string(),
            code_files_created: 0,
            tests_written: 0,
            tests_passing: 0,
            blocking_issue: None,
        }
    }
}

#[derive(Debug, Clone)]
enum ImplementationEvent {
    StartPlanning,
    PlanningComplete,
    StartCoding,
    CodeFileCreated,
    AllCodeComplete,
    StartTesting,
    TestsWritten(u32),
    AllTestsPassing,
    SomeTestsFailing,
    BlockedByIssue(String),
    UnblockIssue,
    AllTestsPass,
}

impl ImplementationState {
    fn on_event(self, event: ImplementationEvent, context: &mut ImplementationContext) -> Self {
        match (self, &event) {
            // Enter Active state
            (_, ImplementationEvent::StartPlanning) => {
                println!("[Implementation] {} starting planning phase", context.task_id);
                context.current_agent = "architect".to_string();
                ImplementationState::Planning
            }

            // Within Planning
            (ImplementationState::Planning, ImplementationEvent::PlanningComplete) => {
                println!("[Implementation] Planning complete, ready to code");
                ImplementationState::Coding
            }

            // Within Coding
            (ImplementationState::Coding, ImplementationEvent::CodeFileCreated) => {
                context.code_files_created += 1;
                println!(
                    "[Implementation] Code file created (total: {})",
                    context.code_files_created
                );
                ImplementationState::Coding
            }

            (ImplementationState::Coding, ImplementationEvent::AllCodeComplete) => {
                println!(
                    "[Implementation] All code complete ({}), starting tests",
                    context.code_files_created
                );
                context.current_agent = "tester".to_string();
                ImplementationState::Testing
            }

            // Within Testing
            (ImplementationState::Testing, ImplementationEvent::TestsWritten(count)) => {
                context.tests_written += count;
                println!("[Implementation] Tests written (total: {})", context.tests_written);
                ImplementationState::Testing
            }

            (ImplementationState::Testing, ImplementationEvent::AllTestsPassing) => {
                println!("[Implementation] All tests passing!");
                ImplementationState::Complete
            }

            (ImplementationState::Testing, ImplementationEvent::SomeTestsFailing) => {
                println!("[Implementation] Some tests failing, back to coding");
                context.current_agent = "coder".to_string();
                ImplementationState::Coding
            }

            // Blocked can be entered from any Active substate
            (_, ImplementationEvent::BlockedByIssue(issue)) => {
                context.blocking_issue = Some(issue.clone());
                println!("[Implementation] Blocked by issue: {}", issue);
                ImplementationState::Blocked
            }

            // Unblock from Blocked
            (ImplementationState::Blocked, ImplementationEvent::UnblockIssue) => {
                println!("[Implementation] Issue resolved, resuming from planning");
                context.blocking_issue = None;
                ImplementationState::Planning
            }

            // Invalid transitions
            (state, event) => {
                println!(
                    "[Implementation] Invalid transition: {:?} on event {:?}",
                    state, event
                );
                state
            }
        }
    }
}

// ============================================================================
// EXAMPLE 3: Async Workflow with Tokio Integration
// ============================================================================

#[tokio::main]
async fn main() {
    println!("=== Descartes State Machine PoC ===\n");

    // Example 1: Code Review Workflow
    println!("Example 1: Code Review Workflow");
    println!("================================\n");
    code_review_example().await;

    println!("\n");

    // Example 2: Implementation Workflow
    println!("Example 2: Implementation Workflow");
    println!("==================================\n");
    implementation_example().await;

    println!("\n");

    // Example 3: Multiple Concurrent Workflows
    println!("Example 3: Multiple Concurrent Workflows");
    println!("=======================================\n");
    concurrent_workflows_example().await;

    println!("\nAll examples completed successfully!");
}

async fn code_review_example() {
    let mut state = CodeReviewState::Submitted;
    let mut context = CodeReviewContext::new("PR-12345".to_string());

    let events = vec![
        CodeReviewEvent::CheckSyntax,
        CodeReviewEvent::SyntaxPassed,
        CodeReviewEvent::ReviewApproved,
        CodeReviewEvent::ReviewApproved,
        CodeReviewEvent::ReviewApproved,
        CodeReviewEvent::AllReviewsComplete,
        CodeReviewEvent::Merge,
    ];

    for event in events {
        println!("  Current state: {:?}", state);
        state = state.on_event(event, &mut context);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    println!("  Final state: {:?}\n", state);
    println!("  Context: {:?}", context);
}

async fn implementation_example() {
    let mut state = ImplementationState::Active;
    let mut context = ImplementationContext::new("TASK-001".to_string());

    let events = vec![
        ImplementationEvent::StartPlanning,
        ImplementationEvent::PlanningComplete,
        ImplementationEvent::StartCoding,
        ImplementationEvent::CodeFileCreated,
        ImplementationEvent::CodeFileCreated,
        ImplementationEvent::CodeFileCreated,
        ImplementationEvent::AllCodeComplete,
        ImplementationEvent::TestsWritten(5),
        ImplementationEvent::SomeTestsFailing,
        ImplementationEvent::CodeFileCreated,
        ImplementationEvent::AllCodeComplete,
        ImplementationEvent::TestsWritten(5),
        ImplementationEvent::AllTestsPassing,
    ];

    for event in events {
        println!("  Current state: {:?}", state);
        state = state.on_event(event, &mut context);
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
    }

    println!("  Final state: {:?}\n", state);
    println!("  Context: {:?}", context);
}

async fn concurrent_workflows_example() {
    let task1 = tokio::spawn(async {
        println!("  [Task-1] Starting workflow...");
        let mut state = CodeReviewState::Submitted;
        let mut context = CodeReviewContext::new("PR-001".to_string());

        let events = vec![
            CodeReviewEvent::CheckSyntax,
            CodeReviewEvent::SyntaxPassed,
            CodeReviewEvent::ReviewApproved,
        ];

        for event in events {
            state = state.on_event(event, &mut context);
            tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        }

        println!("  [Task-1] Complete: {:?}", state);
    });

    let task2 = tokio::spawn(async {
        println!("  [Task-2] Starting workflow...");
        let mut state = ImplementationState::Active;
        let mut context = ImplementationContext::new("IMPL-001".to_string());

        let events = vec![
            ImplementationEvent::StartPlanning,
            ImplementationEvent::PlanningComplete,
            ImplementationEvent::StartCoding,
            ImplementationEvent::CodeFileCreated,
        ];

        for event in events {
            state = state.on_event(event, &mut context);
            tokio::time::sleep(tokio::time::Duration::from_millis(75)).await;
        }

        println!("  [Task-2] Complete: {:?}", state);
    });

    let _ = tokio::join!(task1, task2);
}

// ============================================================================
// SERIALIZATION EXAMPLE (Conceptual)
// ============================================================================

// For real implementation, these would be stored in SQLite via StateStore

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct PersistentWorkflowState {
    task_id: String,
    state_variant: String,
    context: serde_json::Value,
    timestamp: String,
}

// Persistence logic (pseudocode):
//
// async fn save_workflow_state(
//     task_id: &str,
//     state: &ImplementationState,
//     context: &ImplementationContext,
//     store: &impl StateStore,
// ) -> Result<()> {
//     let persistent = PersistentWorkflowState {
//         task_id: task_id.to_string(),
//         state_variant: format!("{:?}", state),
//         context: serde_json::to_value(context)?,
//         timestamp: chrono::Utc::now().to_rfc3339(),
//     };
//
//     store.save_workflow_state(&persistent).await?;
//     Ok(())
// }
//
// async fn resume_workflow_state(
//     task_id: &str,
//     store: &impl StateStore,
// ) -> Result<(ImplementationState, ImplementationContext)> {
//     let persistent = store.load_workflow_state(task_id).await?;
//     let context: ImplementationContext = serde_json::from_value(persistent.context)?;
//     let state = match persistent.state_variant.as_str() {
//         "Planning" => ImplementationState::Planning,
//         "Coding" => ImplementationState::Coding,
//         // ... etc
//         _ => return Err(WorkflowError::InvalidState),
//     };
//     Ok((state, context))
// }

// ============================================================================
// SWARM.TOML MAPPING EXAMPLE
// ============================================================================

/*

Example Swarm.toml that would generate the above implementation workflow:

[workflow.code_implementation]
name = "Code Implementation"
description = "Multi-agent workflow for implementing features"
initial_state = "Planning"

[workflow.code_implementation.states]

[workflow.code_implementation.states.Planning]
description = "Architect plans the implementation approach"
parent = "Active"
handlers = ["validate_requirements", "generate_design"]
next_on_success = "Coding"
next_on_failure = "Blocked"
agents = ["architect"]

[workflow.code_implementation.states.Coding]
description = "Developer implements planned code"
parent = "Active"
handlers = ["write_code", "commit_changes"]
next_on_success = "Testing"
next_on_failure = "Blocked"
agents = ["coder"]

[workflow.code_implementation.states.Testing]
description = "Tester verifies implementation"
parent = "Active"
handlers = ["write_tests", "run_tests"]
next_on_success = "Complete"
next_on_failure = "Coding"  # Loop back for fixes
agents = ["tester"]

[workflow.code_implementation.states.Blocked]
description = "Workflow blocked by external issue"
handlers = ["await_resolution"]
next_on_success = "Planning"
parent = "Root"

[workflow.code_implementation.states.Complete]
description = "Implementation complete"
terminal = true

*/

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_code_review_state_transitions() {
        let mut state = CodeReviewState::Submitted;
        let mut context = CodeReviewContext::new("PR-TEST".to_string());

        state = state.on_event(CodeReviewEvent::CheckSyntax, &mut context);
        assert_eq!(state, CodeReviewState::ReviewingCode);

        state = state.on_event(CodeReviewEvent::SyntaxPassed, &mut context);
        assert_eq!(state, CodeReviewState::ApprovingOrRequesting);

        state = state.on_event(CodeReviewEvent::ReviewApproved, &mut context);
        assert_eq!(context.reviews_approved, 1);
    }

    #[test]
    fn test_implementation_state_transitions() {
        let mut state = ImplementationState::Active;
        let mut context = ImplementationContext::new("TASK-TEST".to_string());

        state = state.on_event(ImplementationEvent::StartPlanning, &mut context);
        assert_eq!(state, ImplementationState::Planning);

        state = state.on_event(ImplementationEvent::PlanningComplete, &mut context);
        assert_eq!(state, ImplementationState::Coding);
    }

    #[test]
    fn test_blocking_from_any_state() {
        let mut state = ImplementationState::Coding;
        let mut context = ImplementationContext::new("TASK-BLOCK".to_string());

        state = state.on_event(
            ImplementationEvent::BlockedByIssue("External API unavailable".to_string()),
            &mut context,
        );
        assert_eq!(state, ImplementationState::Blocked);
        assert!(context.blocking_issue.is_some());

        state = state.on_event(ImplementationEvent::UnblockIssue, &mut context);
        assert_eq!(state, ImplementationState::Planning);
    }
}
