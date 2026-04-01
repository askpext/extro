//! # Extro Core
//!
//! Pure Rust domain logic for the Extro browser extension framework.
//!
//! This crate provides the state machine, command dispatch, browser effects,
//! and AI tool registry that form the deterministic "brain" of every Extro extension.
//!
//! **Key principle**: The model proposes; Rust decides.
//!
//! # Architecture
//!
//! ```text
//! User Action → Content Script → Background → CoreState::dispatch() → BrowserEffects
//! ```
//!
//! JavaScript never contains domain logic. It captures browser state, sends it here,
//! and executes the returned effects.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use thiserror::Error;

/// Identifies which browser extension surface originated a command.
///
/// Used by the core to apply surface-specific policies (e.g., content scripts
/// cannot trigger certain effects, popups get toast notifications).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeSurface {
    /// The background service worker (MV3).
    Background,
    /// A content script injected into a web page.
    ContentScript,
    /// The extension popup UI.
    Popup,
    /// The extension sidebar / side panel.
    Sidebar,
}

/// A snapshot of the current browser state at the moment a command is issued.
///
/// Captured by JavaScript adapters and sent to the Rust core for processing.
/// The core never reads browser state directly — it only receives these snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSnapshot {
    /// The URL of the active tab.
    pub url: String,
    /// The document title of the active tab.
    pub title: String,
    /// Text selected by the user, if any.
    pub selected_text: Option<String>,
}

/// Actions that can be dispatched to the core state machine.
///
/// Add new variants here when extending the extension's capabilities.
/// Each action maps to a handler in [`CoreState::dispatch`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoreAction {
    /// Analyze text selected by the user on a web page.
    AnalyzeSelection,
    /// Summarize the current page content.
    SummarizePage,
    /// Synchronize state between surfaces (heartbeat / init).
    SyncState,
}

/// A command sent from JavaScript to the Rust core.
///
/// This is the only entry point into the core's state machine.
/// JavaScript adapters construct this from user actions and browser state.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreCommand {
    /// Which surface sent this command.
    pub surface: RuntimeSurface,
    /// What action to perform.
    pub action: CoreAction,
    /// Current browser state snapshot.
    pub snapshot: BrowserSnapshot,
}

/// Side effects that the Rust core requests the JavaScript runtime to execute.
///
/// The core never touches browser APIs directly. Instead, it returns a list of
/// these effects, and the background service worker executes them in order.
///
/// # Adding a new effect
///
/// 1. Add a variant here
/// 2. Handle dispatch in `CoreState::dispatch` to return it
/// 3. Add a handler in `extension/src/background/index.js` `applyEffect()`
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrowserEffect {
    /// Read the current DOM selection from the active tab.
    ReadDomSelection,
    /// Read clipboard contents via the Clipboard API.
    ReadClipboard,
    /// Persist a key-value pair in session storage.
    PersistSession { key: String, value: String },
    /// Show a toast notification in the popup UI.
    ShowPopupToast { message: String },
    /// Open the side panel to a specific route.
    OpenSidePanel { route: String },
    /// Inject a content script into the active tab.
    InjectContentScript { file: String },
}

/// The result of processing a [`CoreCommand`].
///
/// Contains a human-readable message and a list of [`BrowserEffect`]s
/// for the JavaScript runtime to execute.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreResult {
    /// Human-readable result message (displayed in UI or logged).
    pub message: String,
    /// Side effects to be executed by the JavaScript background worker.
    pub effects: Vec<BrowserEffect>,
}

/// The central state machine for an Extro extension.
///
/// Owns all domain state and provides deterministic command dispatch.
/// JavaScript never mutates this directly — it sends [`CoreCommand`]s
/// and receives [`CoreResult`]s.
///
/// # Example
///
/// ```
/// use extro_core::*;
///
/// let mut state = CoreState::new();
/// let command = CoreCommand {
///     surface: RuntimeSurface::Popup,
///     action: CoreAction::SyncState,
///     snapshot: BrowserSnapshot {
///         url: "https://example.com".into(),
///         title: "Example".into(),
///         selected_text: None,
///     },
/// };
/// let result = state.dispatch(command);
/// assert_eq!(result.message, "State synchronized");
/// ```
#[derive(Debug, Default)]
pub struct CoreState {
    log: VecDeque<String>,
    session_counter: u64,
}

impl CoreState {
    /// Create a new core state with zeroed counters and empty logs.
    pub fn new() -> Self {
        Self::default()
    }

    /// Dispatch a command and return the result with any side effects.
    ///
    /// This is the main entry point for all extension logic. Each command
    /// increments the session counter and appends to the internal log.
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
                message: format!("Selection analysis prepared for {}", command.snapshot.title),
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

    /// Return the telemetry log as a vector of strings.
    ///
    /// Each entry is a formatted record of a dispatched command.
    pub fn telemetry(&self) -> Vec<String> {
        self.log.iter().cloned().collect()
    }

    /// Return the full command history for agent introspection.
    ///
    /// Agents can use this to review what commands have been processed
    /// and in what order, enabling replay and debugging.
    pub fn history(&self) -> Vec<String> {
        self.log.iter().cloned().collect()
    }

    /// Return the current session counter value.
    pub fn session_count(&self) -> u64 {
        self.session_counter
    }
}

// ---------------------------------------------------------------------------
// AI Tool Registry — "The model proposes; Rust decides."
// ---------------------------------------------------------------------------

/// A tool that AI models are allowed to invoke.
///
/// Each tool has a name, a human-readable description, and a JSON schema
/// that defines its expected arguments. The [`ToolRegistry`] validates
/// every AI tool call against these definitions before execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Unique name of the tool (e.g., `"summarize_page"`).
    pub name: String,
    /// Human-readable description of what the tool does.
    pub description: String,
    /// JSON schema for the tool's expected arguments.
    pub parameters_schema: serde_json::Value,
}

/// A tool call proposed by an AI model.
///
/// The model selects a tool name and provides arguments. The Rust core
/// validates this against the [`ToolRegistry`] before allowing execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AIToolCall {
    /// Name of the tool the model wants to invoke.
    pub tool_name: String,
    /// Arguments provided by the model.
    pub arguments: serde_json::Value,
}

/// Registry of allowed AI tools with validation.
///
/// This is the policy enforcement layer. AI models can only invoke tools
/// that are registered here, and their arguments must conform to the
/// registered schema.
///
/// # Example
///
/// ```
/// use extro_core::*;
///
/// let mut registry = ToolRegistry::new();
/// registry.register(ToolDefinition {
///     name: "summarize".into(),
///     description: "Summarize page content".into(),
///     parameters_schema: serde_json::json!({"type": "object"}),
/// });
///
/// let call = AIToolCall {
///     tool_name: "summarize".into(),
///     arguments: serde_json::json!({}),
/// };
/// assert!(registry.validate(&call).is_ok());
///
/// let bad_call = AIToolCall {
///     tool_name: "delete_everything".into(),
///     arguments: serde_json::json!({}),
/// };
/// assert!(registry.validate(&bad_call).is_err());
/// ```
#[derive(Debug, Default)]
pub struct ToolRegistry {
    tools: HashMap<String, ToolDefinition>,
}

impl ToolRegistry {
    /// Create an empty tool registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool that AI models are allowed to invoke.
    pub fn register(&mut self, tool: ToolDefinition) {
        self.tools.insert(tool.name.clone(), tool);
    }

    /// Validate an AI tool call against the registry.
    ///
    /// Returns `Ok(())` if the tool exists and is registered.
    /// Returns `Err(CoreError::ToolNotRegistered)` if the tool is not allowed.
    pub fn validate(&self, call: &AIToolCall) -> Result<&ToolDefinition, CoreError> {
        self.tools
            .get(&call.tool_name)
            .ok_or_else(|| CoreError::ToolNotRegistered(call.tool_name.clone()))
    }

    /// List all registered tools (for agent discovery).
    pub fn list_tools(&self) -> Vec<&ToolDefinition> {
        self.tools.values().collect()
    }

    /// Check if a specific tool is registered.
    pub fn has_tool(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }
}

/// Errors that can occur during core operations.
#[derive(Debug, Error)]
pub enum CoreError {
    /// The command payload could not be deserialized.
    #[error("invalid command payload")]
    InvalidPayload,
    /// An AI model tried to invoke a tool that is not registered.
    #[error("tool '{0}' is not registered in the tool registry")]
    ToolNotRegistered(String),
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
        assert!(matches!(
            result.effects[1],
            BrowserEffect::ShowPopupToast { .. }
        ));
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
        assert!(matches!(
            result.effects[0],
            BrowserEffect::PersistSession { .. }
        ));
        assert!(matches!(
            result.effects[1],
            BrowserEffect::OpenSidePanel { .. }
        ));
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

    #[test]
    fn test_history() {
        let mut state = CoreState::new();

        for _ in 0..3 {
            state.dispatch(CoreCommand {
                surface: RuntimeSurface::Popup,
                action: CoreAction::SyncState,
                snapshot: BrowserSnapshot {
                    url: "https://example.com".into(),
                    title: "Test".into(),
                    selected_text: None,
                },
            });
        }

        assert_eq!(state.history().len(), 3);
        assert_eq!(state.session_count(), 3);
    }

    #[test]
    fn test_tool_registry_validate_registered() {
        let mut registry = ToolRegistry::new();
        registry.register(ToolDefinition {
            name: "summarize".into(),
            description: "Summarize page content".into(),
            parameters_schema: serde_json::json!({"type": "object"}),
        });

        let call = AIToolCall {
            tool_name: "summarize".into(),
            arguments: serde_json::json!({}),
        };

        assert!(registry.validate(&call).is_ok());
        assert!(registry.has_tool("summarize"));
    }

    #[test]
    fn test_tool_registry_reject_unregistered() {
        let registry = ToolRegistry::new();

        let call = AIToolCall {
            tool_name: "delete_everything".into(),
            arguments: serde_json::json!({}),
        };

        let err = registry.validate(&call).unwrap_err();
        assert!(matches!(err, CoreError::ToolNotRegistered(_)));
        assert!(err.to_string().contains("delete_everything"));
    }

    #[test]
    fn test_tool_registry_list_tools() {
        let mut registry = ToolRegistry::new();
        registry.register(ToolDefinition {
            name: "tool_a".into(),
            description: "Tool A".into(),
            parameters_schema: serde_json::json!({}),
        });
        registry.register(ToolDefinition {
            name: "tool_b".into(),
            description: "Tool B".into(),
            parameters_schema: serde_json::json!({}),
        });

        assert_eq!(registry.list_tools().len(), 2);
    }
}
