use chrono::{DateTime, Utc};
use extro_core::{CoreCommand, CoreResult, CoreState};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub agent_id: String,
    pub timestamp: DateTime<Utc>,
    pub reasoning: Option<String>,
}

#[derive(Default)]
pub struct TraceableEngine {
    state: CoreState,
    traces: Vec<(CoreCommand, ExecutionTrace)>,
}

impl TraceableEngine {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn dispatch_with_trace(
        &mut self,
        command: CoreCommand,
        trace: ExecutionTrace,
    ) -> CoreResult {
        self.traces.push((command.clone(), trace));
        self.state.dispatch(command)
    }

    pub fn get_history(&self) -> &Vec<(CoreCommand, ExecutionTrace)> {
        &self.traces
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use extro_core::{BrowserSnapshot, CoreAction, RuntimeSurface};

    #[test]
    fn test_traceable_dispatch() {
        let mut engine = TraceableEngine::new();
        let command = CoreCommand {
            surface: RuntimeSurface::Popup,
            action: CoreAction::SyncState,
            snapshot: BrowserSnapshot {
                url: "https://example.com".into(),
                title: "Test".into(),
                selected_text: None,
            },
        };
        let trace = ExecutionTrace {
            agent_id: "test-agent".into(),
            timestamp: Utc::now(),
            reasoning: Some("Syncing state for initial load".into()),
        };

        let _result = engine.dispatch_with_trace(command, trace);
        assert_eq!(engine.get_history().len(), 1);
        assert_eq!(engine.get_history()[0].1.agent_id, "test-agent");
    }
}
