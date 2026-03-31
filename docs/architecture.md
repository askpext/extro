# Extro

Extro is a Rust-first framework for production browser extensions. JavaScript is only the shell around browser APIs and rendering. Domain logic, state transitions, and deterministic automation live in Rust and compile to WebAssembly.

## System Diagram

```text
┌──────────────────────────────── Browser Extension Runtime ────────────────────────────────┐
│                                                                                           │
│  ┌───────────────┐     runtime message      ┌──────────────────┐                          │
│  │ Popup / UI    │ ───────────────────────► │ Background SW    │                          │
│  │ React/JS thin │ ◄─────────────────────── │ orchestration hub │                          │
│  └──────┬────────┘      state/effects       └────────┬─────────┘                          │
│         │                                            │                                    │
│         │ direct wasm reads                           │ owns browser API execution         │
│         ▼                                            ▼                                    │
│  ┌──────────────────┐    serialized command    ┌──────────────────┐                       │
│  │ Content Script   │ ───────────────────────► │ Rust/WASM Core   │                       │
│  │ DOM capture only │ ◄─────────────────────── │ reducer + policy │                       │
│  └────────┬─────────┘      deterministic plan  └────────┬─────────┘                       │
│           │                                             │                                 │
│           │ DOM snapshot                                │ effect list                     │
│           ▼                                             ▼                                 │
│    Web page DOM / selection                    Browser APIs via JS adapters               │
│                                                                                           │
└───────────────────────────────────────────────────────────────────────────────────────────┘

CLI (`extro`) drives scaffolding, build, watch, wasm packaging, and local browser injection.
```

## Roles

### Background Script

The background service worker is the sole orchestration boundary. It owns:

- extension-wide message routing
- browser API execution
- storage writes
- alarm, tab, and network side effects
- AI tool-call scheduling

It does **not** contain business logic. It converts incoming runtime messages into `CoreCommand`, sends them into WASM, then executes the returned `BrowserEffect` list.

### Content Script

The content script is an untrusted page adapter. It owns:

- DOM reads
- event capture
- page metadata extraction
- optional DOM patch application approved by the background layer

It must never decide business policy. It sends browser state snapshots to the background script.

### Popup / UI

The popup is a thin render surface. It owns:

- rendering state already computed by Rust
- collecting explicit user intent
- displaying deterministic action results

No reducer logic lives here. UI sends commands and renders `CoreResult`.

### Rust / WASM Core

The core is the product. It owns:

- state machines
- command validation
- routing policy
- AI planning constraints
- serialization contracts
- deterministic effect generation

The Rust core never touches browser APIs directly. It emits effects; the JS runtime executes them.

## Data Flow

1. A user clicks popup or selects text in a page.
2. UI/content script captures browser state snapshot.
3. Snapshot is sent to background via `chrome.runtime.sendMessage`.
4. Background passes payload to `WasmEngine.dispatch`.
5. Rust validates command, updates state, returns `CoreResult { message, effects }`.
6. Background executes each effect through browser APIs.
7. Result is streamed back to UI/content surfaces for rendering.

## Monorepo Structure

```text
/Cargo.toml                 Rust workspace root
/crates/core                Pure Rust domain engine, state, effect model
/crates/wasm                wasm-bindgen bridge around core
/cli                        Rust CLI for scaffolding/build/watch/dev-inject
/extension                  Manifest V3 runtime shell
/extension/src/background   Message router + browser API adapter
/extension/src/content      DOM/event adapters
/extension/src/popup        Thin UI shell
/extension/src/shared       WASM loader + shared transport helpers
/examples                   Example extensions generated from templates
/docs                       Framework design, RFCs, operating guides
```

## Rust + WASM Integration

### Minimal Rust export

```rust
#[wasm_bindgen(js_name = classifyUrl)]
pub fn classify_url(url: &str) -> String {
    if url.contains("github.com") { "developer".into() } else { "general".into() }
}
```

### Stateful Rust engine exposed to JS

```rust
#[wasm_bindgen]
pub struct WasmEngine {
    inner: CoreState,
}

#[wasm_bindgen]
impl WasmEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self { inner: CoreState::new() }
    }

    pub fn dispatch(&mut self, input: JsValue) -> Result<JsValue, JsValue> {
        let command: CoreCommand = serde_wasm_bindgen::from_value(input)?;
        let result = self.inner.dispatch(command);
        serde_wasm_bindgen::to_value(&result).map_err(Into::into)
    }
}
```

### JS calling WASM

```js
import init, { WasmEngine } from "../../../pkg/extro_wasm.js";

await init();
const engine = new WasmEngine();

const result = engine.dispatch({
  surface: "Popup",
  action: "SummarizePage",
  snapshot: { url: tab.url, title: tab.title, selected_text: null }
});
```

### State model

State stays inside Rust as a long-lived reducer-owned object. JS never mutates core state directly. JS passes facts; Rust returns decisions.

### Cross-surface messaging

Content and popup never talk to each other directly. All messages terminate at background:

```text
content/popup -> background -> wasm core -> browser effect executor -> background -> surface
```

That keeps transport deterministic and debuggable.

## CLI Design

Public entrypoint:

```text
extro new my-extension
extro build
extro watch
extro dev-inject chromium
```

Responsibilities:

- `new`: scaffold workspace, manifest, typed contracts, Rust crates, example commands
- `build`: compile `extro-core`, build `extro-wasm` with `wasm-pack`, bundle `extension`
- `watch`: incremental Rust rebuild plus extension asset rebuild, then notify service worker reload
- `dev-inject`: launch Chromium/Chrome with `--load-extension` pointed at the build output

`npx create-extro-app` should just be a tiny npm shim that downloads or delegates to the Rust binary. The product remains Rust-native.

## Developer Experience

### Hot Reload

Use a two-lane loop:

- lane 1: `cargo watch` or custom notify-based watcher rebuilds the wasm package
- lane 2: extension bundler rebuilds JS/UI assets

When either lane completes, the CLI touches a reload marker and reconnects the extension through the DevTools protocol. Background state is rehydrated from session storage to hide MV3 worker churn.

### Logging

Three structured channels:

- `core`: emitted from Rust, serialized as JSON events
- `runtime`: background/content/popup adapter logs
- `browser`: explicit browser API effect execution logs

Every command gets a correlation id so a single user action can be followed across popup, background, wasm, and browser effect execution.

### Error Handling

- Rust errors are typed and serializable
- JS adapters never throw raw exceptions across surfaces
- every runtime boundary returns `{ code, message, context, correlation_id }`
- failed effects are retried only if marked idempotent

### Testing

Ship four layers:

1. Rust unit tests for reducers and policy
2. contract tests for JSON payloads between JS and WASM
3. headless browser integration tests for background/content behavior
4. fixture-based AI planner tests to guarantee deterministic tool selection

## AI Integration Layer

AI does not call browser APIs directly. It is a planner attached to the Rust core.

### Model

- browser state is observed by JS adapters
- normalized facts are passed into Rust
- Rust exposes a strict tool registry
- AI selects from Rust-defined tools
- Rust validates arguments and emits browser effects

### Deterministic loop

```text
User selects text
-> content script captures selection + URL
-> background sends CoreCommand::AnalyzeSelection
-> Rust builds AI context and allowed tool list
-> AI returns tool call request (for example: summarize_selection)
-> Rust validates tool call against schema and policy
-> Rust emits BrowserEffect::PersistSession + ShowPopupToast
-> background executes effects
-> popup renders result
```

The key rule: the model proposes; Rust decides.

## MVP Scope

Build in week 1:

- workspace + CLI
- one Rust core crate
- wasm bridge
- MV3 extension shell
- popup + content + background messaging
- one deterministic command pipeline
- one AI tool-call loop with mocked model provider
- build/watch/dev-inject flow

Ignore initially:

- Firefox support
- sidebar UI
- remote extension registry
- plugin marketplace
- CRDT sync
- full React app generator

Usable in one week means a developer can scaffold, run watch mode, select text on any page, route it through Rust, and see deterministic output in the extension UI.

## Positioning

### Taglines

1. Extro: browser extensions with a Rust brain.
2. Extro: the clean architecture framework for serious extensions.
3. Extro: build MV3 extensions like systems software.

### Difference from existing tools

- existing extension starters optimize for speed to first popup; Extro optimizes for long-term maintainability
- existing tools keep logic in TypeScript; Extro centralizes logic and state in Rust
- existing AI integrations let models act directly; Extro inserts a deterministic Rust policy layer

### Target Audience

- teams building commercial browser extensions
- Rust engineers who do not want extension logic trapped in frontend code
- AI product teams that need auditable browser-side automation

