# Page Summarizer Example

A simple Extro extension that summarizes web pages using AI.

## Features

- Select text on any page to get instant analysis
- Click popup button to summarize entire page
- Shows URL classification (developer, documentation, general)

## Quick Start

```bash
# From the root directory
cd examples/page-summarizer

# Install dependencies
pnpm install

# Build and watch
extro watch

# Launch Chrome with extension
extro dev-inject chrome
```

## How It Works

### 1. Rust Core (`crates/core/src/lib.rs`)

Defines the state machine and effects:

```rust
pub enum CoreAction {
    AnalyzeSelection,
    SummarizePage,
}

pub enum BrowserEffect {
    ShowPopupToast { message: String },
    OpenSidePanel { route: String },
    PersistSession { key: String, value: String },
}
```

### 2. Content Script (`extension/src/content/index.js`)

Captures user selections:

```js
document.addEventListener("mouseup", async () => {
  const selection = window.getSelection().toString().trim();
  if (!selection) return;

  const result = await chrome.runtime.sendMessage({
    type: "extro.command",
    payload: {
      surface: "ContentScript",
      action: "AnalyzeSelection",
      snapshot: {
        url: window.location.href,
        title: document.title,
        selected_text: selection
      }
    }
  });
});
```

### 3. Background Service Worker (`extension/src/background/index.js`)

Orchestrates WASM execution and effect handling:

```js
import { runCore } from "../shared/engine.js";

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type !== "extro.command") return false;

  runCore(message.payload)
    .then((result) => {
      for (const effect of result.effects) {
        applyEffect(effect, sender);
      }
      sendResponse(result);
    });

  return true;
});
```

## Project Structure

```
examples/page-summarizer/
├── Cargo.toml              # Rust workspace
├── crates/
│   ├── core/               # Pure Rust logic
│   └── wasm/               # WASM bindings
├── extension/
│   ├── manifest.json       # Manifest V3
│   └── src/
│       ├── background/     # Service worker
│       ├── content/        # Content script
│       ├── popup/          # Popup UI
│       └── shared/         # WASM loader
└── README.md
```

## Customization

### Add New Actions

1. Add to `CoreAction` enum in `crates/core/src/lib.rs`
2. Handle in `CoreState::dispatch()`
3. Call from JavaScript via `chrome.runtime.sendMessage()`

### Add New Effects

1. Add to `BrowserEffect` enum
2. Handle in `extension/src/background/index.js` `applyEffect()` function

## License

MIT
