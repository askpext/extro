//! # Extro Agent
//!
//! Agentic bridge for the Extro framework, providing traceability
//! and LLM-friendly execution patterns.
//!
//! This crate wraps [`extro_core::CoreState`] with an execution trace
//! so every command dispatched by an AI agent is recorded with its
//! reasoning, timestamp, and agent identity.

use chrono::{DateTime, Utc};
use extro_core::{CoreCommand, CoreResult, CoreState};
use serde::{Deserialize, Serialize};

/// A record of why and when an AI agent dispatched a command.
///
/// Attached to every command for audit trails and debugging.
/// Agents should always provide `reasoning` to explain their intent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTrace {
    /// Unique identifier for the agent that dispatched this command.
    pub agent_id: String,
    /// When the command was dispatched.
    pub timestamp: DateTime<Utc>,
    /// The agent's reasoning for dispatching this command (optional but encouraged).
    pub reasoning: Option<String>,
}

/// A wrapper around [`CoreState`] that records execution traces.
///
/// Use this instead of `CoreState` directly when building AI-powered
/// extensions that need auditability.
///
/// # Example
///
/// ```
/// use extro_agent::*;
/// use extro_core::*;
/// use chrono::Utc;
///
/// let mut engine = TraceableEngine::new();
/// let command = CoreCommand {
///     surface: RuntimeSurface::Popup,
///     action: CoreAction::SyncState,
///     snapshot: BrowserSnapshot {
///         url: "https://example.com".into(),
///         title: "Test".into(),
///         selected_text: None,
///     },
/// };
/// let trace = ExecutionTrace {
///     agent_id: "my-agent".into(),
///     timestamp: Utc::now(),
///     reasoning: Some("Initial state sync".into()),
/// };
///
/// let result = engine.dispatch_with_trace(command, trace);
/// assert_eq!(engine.get_history().len(), 1);
/// ```
#[derive(Default)]
pub struct TraceableEngine {
    state: CoreState,
    traces: Vec<(CoreCommand, ExecutionTrace)>,
}

impl TraceableEngine {
    /// Create a new traceable engine with fresh state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Dispatch a command with an attached execution trace.
    ///
    /// The command and trace are recorded in the history before
    /// the command is forwarded to the inner [`CoreState`].
    pub fn dispatch_with_trace(
        &mut self,
        command: CoreCommand,
        trace: ExecutionTrace,
    ) -> CoreResult {
        self.traces.push((command.clone(), trace));
        self.state.dispatch(command)
    }

    /// Get the full history of commands and their execution traces.
    pub fn get_history(&self) -> &Vec<(CoreCommand, ExecutionTrace)> {
        &self.traces
    }

    /// Clear all recorded traces.
    ///
    /// Useful for long-running agents that need to free memory
    /// after persisting traces to external storage.
    pub fn clear_history(&mut self) {
        self.traces.clear();
    }

    /// Filter traces by a specific agent ID.
    ///
    /// Returns only the traces belonging to the specified agent,
    /// useful in multi-agent scenarios.
    pub fn filter_by_agent(&self, agent_id: &str) -> Vec<&(CoreCommand, ExecutionTrace)> {
        self.traces
            .iter()
            .filter(|(_, trace)| trace.agent_id == agent_id)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use extro_core::{BrowserSnapshot, CoreAction, RuntimeSurface};

    fn make_command() -> CoreCommand {
        CoreCommand {
            surface: RuntimeSurface::Popup,
            action: CoreAction::SyncState,
            snapshot: BrowserSnapshot {
                url: "https://example.com".into(),
                title: "Test".into(),
                selected_text: None,
            },
        }
    }

    fn make_trace(agent_id: &str) -> ExecutionTrace {
        ExecutionTrace {
            agent_id: agent_id.into(),
            timestamp: Utc::now(),
            reasoning: Some("test reasoning".into()),
        }
    }

    #[test]
    fn test_traceable_dispatch() {
        let mut engine = TraceableEngine::new();
        let _result = engine.dispatch_with_trace(make_command(), make_trace("test-agent"));
        assert_eq!(engine.get_history().len(), 1);
        assert_eq!(engine.get_history()[0].1.agent_id, "test-agent");
    }

    #[test]
    fn test_clear_history() {
        let mut engine = TraceableEngine::new();
        engine.dispatch_with_trace(make_command(), make_trace("agent-1"));
        engine.dispatch_with_trace(make_command(), make_trace("agent-2"));
        assert_eq!(engine.get_history().len(), 2);

        engine.clear_history();
        assert!(engine.get_history().is_empty());
    }

    #[test]
    fn test_filter_by_agent() {
        let mut engine = TraceableEngine::new();
        engine.dispatch_with_trace(make_command(), make_trace("agent-a"));
        engine.dispatch_with_trace(make_command(), make_trace("agent-b"));
        engine.dispatch_with_trace(make_command(), make_trace("agent-a"));

        let filtered = engine.filter_by_agent("agent-a");
        assert_eq!(filtered.len(), 2);

        let filtered_b = engine.filter_by_agent("agent-b");
        assert_eq!(filtered_b.len(), 1);
    }
}
