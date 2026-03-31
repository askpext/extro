use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeSurface {
    Background,
    ContentScript,
    Popup,
    Sidebar,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSnapshot {
    pub url: String,
    pub title: String,
    pub selected_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoreAction {
    AnalyzeSelection,
    SummarizePage,
    SyncState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreCommand {
    pub surface: RuntimeSurface,
    pub action: CoreAction,
    pub snapshot: BrowserSnapshot,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrowserEffect {
    ReadDomSelection,
    PersistSession { key: String, value: String },
    ShowPopupToast { message: String },
    OpenSidePanel { route: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreResult {
    pub message: String,
    pub effects: Vec<BrowserEffect>,
}

#[derive(Debug, Default)]
pub struct CoreState {
    log: VecDeque<String>,
    session_counter: u64,
}

impl CoreState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn dispatch(&mut self, command: CoreCommand) -> CoreResult {
        self.session_counter += 1;
        self.log.push_back(format!(
            "#{} {:?} on {}",
            self.session_counter, command.action, command.snapshot.url
        ));

        if self.log.len() > 100 {
            self.log.pop_front();
        }

        match command.action {
            CoreAction::AnalyzeSelection => CoreResult {
                message: format!(
                    "Selection analysis prepared for {}",
                    command.snapshot.title
                ),
                effects: vec![
                    BrowserEffect::ReadDomSelection,
                    BrowserEffect::ShowPopupToast {
                        message: "Selection sent to AI pipeline".into(),
                    },
                ],
            },
            CoreAction::SummarizePage => CoreResult {
                message: format!("Summary job queued for {}", command.snapshot.url),
                effects: vec![
                    BrowserEffect::PersistSession {
                        key: "last_summary_url".into(),
                        value: command.snapshot.url,
                    },
                    BrowserEffect::OpenSidePanel {
                        route: "/jobs/latest".into(),
                    },
                ],
            },
            CoreAction::SyncState => CoreResult {
                message: "State synchronized".into(),
                effects: vec![],
            },
        }
    }

    pub fn telemetry(&self) -> Vec<String> {
        self.log.iter().cloned().collect()
    }
}

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("invalid command payload")]
    InvalidPayload,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_core_state_new() {
        let state = CoreState::new();
        assert_eq!(state.session_counter, 0);
        assert!(state.log.is_empty());
    }

    #[test]
    fn test_dispatch_analyze_selection() {
        let mut state = CoreState::new();
        let command = CoreCommand {
            surface: RuntimeSurface::ContentScript,
            action: CoreAction::AnalyzeSelection,
            snapshot: BrowserSnapshot {
                url: "https://example.com".to_string(),
                title: "Test Page".to_string(),
                selected_text: Some("Hello World".to_string()),
            },
        };

        let result = state.dispatch(command);

        assert!(result.message.contains("Selection analysis"));
        assert_eq!(result.effects.len(), 2);
        assert!(matches!(result.effects[0], BrowserEffect::ReadDomSelection));
        assert!(matches!(result.effects[1], BrowserEffect::ShowPopupToast { .. }));
    }

    #[test]
    fn test_dispatch_summarize_page() {
        let mut state = CoreState::new();
        let command = CoreCommand {
            surface: RuntimeSurface::Popup,
            action: CoreAction::SummarizePage,
            snapshot: BrowserSnapshot {
                url: "https://docs.example.com".to_string(),
                title: "Docs".to_string(),
                selected_text: None,
            },
        };

        let result = state.dispatch(command);

        assert!(result.message.contains("Summary job queued"));
        assert_eq!(result.effects.len(), 2);
        assert!(matches!(result.effects[0], BrowserEffect::PersistSession { .. }));
        assert!(matches!(result.effects[1], BrowserEffect::OpenSidePanel { .. }));
    }

    #[test]
    fn test_dispatch_sync_state() {
        let mut state = CoreState::new();
        let command = CoreCommand {
            surface: RuntimeSurface::Background,
            action: CoreAction::SyncState,
            snapshot: BrowserSnapshot {
                url: "https://example.com".to_string(),
                title: "Test".to_string(),
                selected_text: None,
            },
        };

        let result = state.dispatch(command);

        assert_eq!(result.message, "State synchronized");
        assert!(result.effects.is_empty());
    }

    #[test]
    fn test_session_counter_increments() {
        let mut state = CoreState::new();

        for i in 1..=5 {
            let command = CoreCommand {
                surface: RuntimeSurface::Popup,
                action: CoreAction::SyncState,
                snapshot: BrowserSnapshot {
                    url: "https://example.com".to_string(),
                    title: "Test".to_string(),
                    selected_text: None,
                },
            };
            state.dispatch(command);
            assert_eq!(state.session_counter, i);
        }
    }

    #[test]
    fn test_log_truncation() {
        let mut state = CoreState::new();

        // Dispatch 150 commands to test log truncation
        for _ in 0..150 {
            let command = CoreCommand {
                surface: RuntimeSurface::Popup,
                action: CoreAction::SyncState,
                snapshot: BrowserSnapshot {
                    url: "https://example.com".to_string(),
                    title: "Test".to_string(),
                    selected_text: None,
                },
            };
            state.dispatch(command);
        }

        // Log should be truncated to 100 entries
        assert!(state.log.len() <= 100);
    }

    #[test]
    fn test_telemetry() {
        let mut state = CoreState::new();
        let command = CoreCommand {
            surface: RuntimeSurface::Popup,
            action: CoreAction::SyncState,
            snapshot: BrowserSnapshot {
                url: "https://example.com".to_string(),
                title: "Test".to_string(),
                selected_text: None,
            },
        };
        state.dispatch(command);

        let telemetry = state.telemetry();
        assert_eq!(telemetry.len(), 1);
        assert!(telemetry[0].contains("SyncState"));
    }

    #[test]
    fn test_browser_snapshot_serialization() {
        let snapshot = BrowserSnapshot {
            url: "https://example.com".to_string(),
            title: "Test Page".to_string(),
            selected_text: Some("Selected text".to_string()),
        };

        let serialized = serde_json::to_string(&snapshot).unwrap();
        let deserialized: BrowserSnapshot = serde_json::from_str(&serialized).unwrap();

        assert_eq!(snapshot.url, deserialized.url);
        assert_eq!(snapshot.title, deserialized.title);
        assert_eq!(snapshot.selected_text, deserialized.selected_text);
    }
}
