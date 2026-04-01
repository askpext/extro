# AGENTS.md — Extro Framework

> This file is for AI coding agents (Claude Code, Gemini CLI, Cursor, Codex, Qwen, etc.).
> It tells you everything you need to build browser extensions with Extro.

## What is Extro?

Extro is a **Rust-first browser extension framework**. Domain logic lives in Rust (compiled to WebAssembly). JavaScript is a thin adapter for browser APIs and UI rendering.

**Use Extro when**: Building a Chrome/Chromium browser extension (MV3) where you want type-safe, testable logic.

## Quick Start (for agents)

```bash
# Install the CLI
cargo install extro-cli

# Create a new extension
extro new my-extension
cd my-extension

# ⚠️ CRITICAL: Install JS dependencies (includes @askpext/runtime)
pnpm install

# Build the extension (compiles Rust → WASM → extension bundle)
extro build

# Run tests
extro test

# Launch Chrome with extension loaded
extro dev-inject chrome
```

> **CRITICAL**: You MUST run `pnpm install` before `extro build`. The scaffolded `package.json` depends on `@askpext/runtime` for the JS runtime adapter (WASM loader, structured logging). Skipping this will cause import errors.

## Architecture Rules

**The reducer pattern**: `CoreCommand → CoreState::dispatch() → CoreResult + BrowserEffects`

| Layer | Location | Responsibility |
|-------|----------|----------------|
| **Rust Core** | `crates/core/src/lib.rs` | ALL domain logic, state, validation, AI policy |
| **WASM Bridge** | `crates/wasm/src/lib.rs` | Thin wasm-bindgen wrapper around core |
| **Agent Tracing** | `crates/agent/src/lib.rs` | Execution traces for AI audit trails |
| **Background SW** | `extension/src/background/index.js` | Message routing + effect execution |
| **Content Script** | `extension/src/content/index.js` | DOM reading + event capture |
| **Popup UI** | `extension/src/popup/` | Thin render surface |
| **WASM Loader** | `extension/src/shared/engine.js` | Initializes and caches WasmEngine |

### Hard Rules

1. **NEVER put business logic in JavaScript**. JS captures browser state and executes effects.
2. **NEVER call browser APIs from Rust**. Rust receives snapshots and returns decisions.
3. **ALL state transitions happen in `CoreState::dispatch()`**.
4. **ALL side effects are expressed as `BrowserEffect` variants**.
5. **AI tool calls are validated through `ToolRegistry`** before execution.

## File Map

```
Cargo.toml                          # Rust workspace root
package.json                        # npm deps (includes @askpext/runtime)
AGENTS.md                           # This file — agent instructions
crates/core/Cargo.toml              # Core crate config
crates/core/src/lib.rs              # ⭐ Main domain logic (START HERE)
crates/wasm/Cargo.toml              # WASM crate config
crates/wasm/src/lib.rs              # wasm-bindgen bridge
crates/agent/Cargo.toml             # Agent crate config
crates/agent/src/lib.rs             # TraceableEngine for AI agents
cli/Cargo.toml                      # CLI crate config
cli/src/main.rs                     # CLI implementation
extension/manifest.json             # Chrome MV3 manifest
extension/src/background/index.js   # Service worker (effect executor)
extension/src/content/index.js      # Content script (DOM adapter)
extension/src/popup/index.html      # Popup HTML
extension/src/popup/index.js        # Popup JS
extension/src/shared/engine.js      # WASM loader + engine singleton
npm/runtime/src/index.js            # @askpext/runtime npm package
npm/runtime/src/logger.js           # Structured logging
```

## Standard Workflows

### Adding a New User Action

1. Add a variant to `CoreAction` enum in `crates/core/src/lib.rs`
2. Add a match arm in `CoreState::dispatch()` returning a `CoreResult`
3. Define any new `BrowserEffect` variants needed
4. Handle effects in `extension/src/background/index.js` `applyEffect()`
5. If the action needs browser data, add gathering logic to `enrichPayload()` in background.js
6. Trigger from JS: `chrome.runtime.sendMessage({ type: "extro.command", payload: { surface, action: "YourAction", snapshot } })`
7. Run `extro test` then `extro build`

### Adding a Browser Effect

1. Add a variant to `BrowserEffect` enum in `crates/core/src/lib.rs`
2. Return it from the appropriate action in `CoreState::dispatch()`
3. Add a case in `extension/src/background/index.js` `applyEffect()`
4. Run `extro test` then `extro build`

### Adding an AI Tool

1. Create a `ToolDefinition` with name, description, and JSON schema
2. Register it with `ToolRegistry::register()`
3. When AI proposes a call, validate with `ToolRegistry::validate(&call)`
4. If valid, generate appropriate `BrowserEffect`s
5. Log via `TraceableEngine::dispatch_with_trace()`

## Effect Executor Pattern

The background script receives effects from Rust and executes them. **This is the correct async pattern**:

```js
// In extension/src/background/index.js
chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type !== "extro.command") return false;

  (async () => {
    try {
      const result = await runCore(message.payload);
      const effectResults = [];
      for (const effect of (result.effects || [])) {
        const effectResult = await applyEffect(effect);
        effectResults.push(effectResult);
      }
      sendResponse({ ...result, effectResults });
    } catch (err) {
      sendResponse({ error: err.message });
    }
  })();

  return true; // keep message channel open for async response
});

async function applyEffect(effect) {
  if (effect.ShowPopupToast) {
    return { toast: effect.ShowPopupToast.message };
  }
  if (effect.YourCustomEffect) {
    const data = await chrome.someApi.someMethod();
    return { data };
  }
  console.warn('[extro] unhandled effect:', effect);
  return {};
}
```

## Enrich Before Dispatch (Data-In Pattern)

When your extension needs data FROM the browser (tabs, bookmarks, history, storage, DOM, etc.), **gather it in JS and pass it to Rust via `snapshot.context`**:

```js
// In enrichPayload() — runs BEFORE every WASM call
async function enrichPayload(payload) {
  if (payload.action === 'ListTabs') {
    payload.snapshot.context = {
      tabs: await chrome.tabs.query({})
    };
  }
  if (payload.action === 'GetBookmarks') {
    payload.snapshot.context = {
      bookmarks: await chrome.bookmarks.getTree()
    };
  }
  return payload;
}
```

```rust
// In Rust — parse context data and apply logic
CoreAction::ListTabs => {
    let tabs: Vec<TabInfo> = serde_json::from_value(
        command.snapshot.context["tabs"].clone()
    ).unwrap_or_default();
    // Sort, filter, search — all in Rust
}
```

**This pattern ensures Rust owns ALL logic** while JS remains a thin data-gathering + effect-executing layer.

## Common Mistakes (AVOID)

| ❌ Mistake | ✅ Correct |
|-----------|-----------|
| Calling `chrome.*` APIs from popup JS | Send `extro.command` → let background handle effects |
| Skipping `pnpm install` before build | Always run `pnpm install` first (installs `@askpext/runtime`) |
| Business logic in JavaScript | All logic in `crates/core/src/lib.rs` |
| Forgetting `applyEffect()` handler | Every new `BrowserEffect` needs a case in `applyEffect()` |
| Using `.then(sendResponse)` for async | Wrap in `(async () => { ... })()` + `return true` |
| Fetching browser data in Rust | Gather in JS, pass via `snapshot.context`, process in Rust |

## Key Types (Rust)

```rust
// Browser state snapshot (sent from JS to Rust)
struct BrowserSnapshot {
    url: String,
    title: String,
    selected_text: Option<String>,
    context: serde_json::Value,  // tabs, bookmarks, history, storage, etc.
}

// Input to the state machine
struct CoreCommand { surface: RuntimeSurface, action: CoreAction, snapshot: BrowserSnapshot }

// Output from the state machine
struct CoreResult { message: String, effects: Vec<BrowserEffect> }

// Side effects JS must execute
enum BrowserEffect { ReadDomSelection, ReadClipboard, PersistSession{..}, ShowPopupToast{..}, OpenSidePanel{..}, InjectContentScript{..} }

// AI policy enforcement
struct ToolRegistry { /* register(), validate(), list_tools(), has_tool() */ }
struct AIToolCall { tool_name: String, arguments: serde_json::Value }
struct ToolDefinition { name: String, description: String, parameters_schema: serde_json::Value }

// Agent tracing
struct TraceableEngine { /* dispatch_with_trace(), get_history(), filter_by_agent() */ }
struct ExecutionTrace { agent_id: String, timestamp: DateTime<Utc>, reasoning: Option<String> }
```

## CLI Commands

```
extro new <name>               Create a new extension project
extro build                    Build for production (Rust → WASM → extension bundle)
extro build --dev              Build with debug symbols
extro watch                    Watch mode with auto-rebuild
extro dev-inject <browser>     Launch browser with extension loaded
extro test [package]           Run Rust + WASM tests
extro clean                    Clean all build artifacts
extro package                  Package as .zip for Chrome Web Store
extro info                     Show environment info
extro assistant status         Machine-readable project status (JSON)
extro assistant suggest        Get next-step recommendations
extro assistant explain <topic> Explain a concept (architecture, effects, commands, wasm-bridge, tool-registry)
```

## Message Protocol

All inter-surface communication uses a single message format:

```js
chrome.runtime.sendMessage({
  type: "extro.command",
  payload: {
    surface: "Popup" | "ContentScript" | "Background" | "Sidebar",
    action: "AnalyzeSelection" | "SummarizePage" | "SyncState",
    snapshot: { url: "...", title: "...", selected_text: "..." | null }
  }
});
```

## Testing

```bash
extro test          # Run all workspace tests
extro test core     # Run only core crate tests
extro test wasm     # Run only WASM crate tests
cargo test -p extro-agent  # Run agent crate tests
```

