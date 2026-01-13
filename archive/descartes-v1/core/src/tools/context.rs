//! Execution context for session-aware tools.

use uuid::Uuid;

/// Context passed to tool executors for session-aware operations.
///
/// This struct provides session and agent identification to tool executors,
/// enabling tools like Swank to look up session-specific clients from a registry.
#[derive(Clone, Debug)]
pub struct ExecutionContext {
    /// Unique session identifier
    pub session_id: Uuid,
    /// Agent identifier (may be same as session_id)
    pub agent_id: Uuid,
}

impl ExecutionContext {
    /// Create a new execution context.
    pub fn new(session_id: Uuid, agent_id: Uuid) -> Self {
        Self {
            session_id,
            agent_id,
        }
    }

    /// Create context where session_id equals agent_id.
    pub fn for_agent(agent_id: Uuid) -> Self {
        Self {
            session_id: agent_id,
            agent_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_context_new() {
        let session_id = Uuid::new_v4();
        let agent_id = Uuid::new_v4();
        let ctx = ExecutionContext::new(session_id, agent_id);

        assert_eq!(ctx.session_id, session_id);
        assert_eq!(ctx.agent_id, agent_id);
    }

    #[test]
    fn test_execution_context_for_agent() {
        let agent_id = Uuid::new_v4();
        let ctx = ExecutionContext::for_agent(agent_id);

        assert_eq!(ctx.session_id, agent_id);
        assert_eq!(ctx.agent_id, agent_id);
    }

    #[test]
    fn test_execution_context_clone() {
        let ctx = ExecutionContext::for_agent(Uuid::new_v4());
        let cloned = ctx.clone();

        assert_eq!(ctx.session_id, cloned.session_id);
        assert_eq!(ctx.agent_id, cloned.agent_id);
    }
}
