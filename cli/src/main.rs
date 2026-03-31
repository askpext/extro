use anyhow::{bail, Context, Result};
use clap::{Parser, Subcommand};
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};
use std::thread;
use std::time::{Duration, SystemTime};

#[derive(Parser)]
#[command(
    name = "extro",
    about = "Rust-first browser extension framework CLI",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Create a new Extro extension project
    New {
        /// Name of the extension
        name: String,
        /// Template to use (minimal, full, ai)
        #[arg(short, long, default_value = "minimal")]
        template: String,
    },
    /// Build the extension for production
    Build {
        /// Build for development (includes debug symbols)
        #[arg(short, long)]
        dev: bool,
    },
    /// Watch for changes and rebuild automatically
    Watch,
    /// Launch browser with extension loaded for development
    DevInject {
        /// Browser to use (chrome, chromium, edge, brave)
        #[arg(default_value = "chrome")]
        browser: String,
    },
    /// Run tests for the Rust core and WASM
    Test {
        /// Test package (core, wasm, all)
        #[arg(default_value = "all")]
        package: String,
    },
    /// Clean build artifacts
    Clean,
    /// Package extension for distribution (.zip, .crx)
    Package {
        /// Output format (zip, crx)
        #[arg(short, long, default_value = "zip")]
        format: String,
    },
    /// Display build information and environment
    Info,
    /// Agent-specific commands for machine consumption
    Assistant {
        #[command(subcommand)]
        subcommand: AssistantCommand,
    },
}

#[derive(Subcommand)]
enum AssistantCommand {
    /// Show current project status in agent-friendly format
    Status {
        /// Output format (text, json)
        #[arg(short, long, default_value = "json")]
        format: String,
    },
    /// Suggest next steps for the agent based on codebase analysis
    Suggest,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::New { name, template } => scaffold(&name, &template),
        Command::Build { dev } => build_workspace(dev),
        Command::Watch => watch_workspace(),
        Command::DevInject { browser } => dev_inject(&browser),
        Command::Test { package } => run_tests(&package),
        Command::Clean => clean_workspace(),
        Command::Package { format } => package_extension(&format),
        Command::Info => show_info(),
        Command::Assistant { subcommand } => match subcommand {
            AssistantCommand::Status { format } => assistant_status(&format),
            AssistantCommand::Suggest => assistant_suggest(),
        },
    }
}

fn scaffold(name: &str, template: &str) -> Result<()> {
    println!(
        "Creating new Extro extension '{}' with '{}' template...",
        name, template
    );

    let root = PathBuf::from(name);

    // Create directory structure
    fs::create_dir_all(root.join("extension/src/background"))
        .with_context(|| format!("failed to create {}", root.display()))?;
    fs::create_dir_all(root.join("extension/src/content"))?;
    fs::create_dir_all(root.join("extension/src/popup"))?;
    fs::create_dir_all(root.join("extension/src/shared"))?;
    fs::create_dir_all(root.join("crates/core/src"))?;
    fs::create_dir_all(root.join("crates/wasm/src"))?;
    fs::create_dir_all(root.join("crates/agent/src"))?;
    fs::create_dir_all(root.join(".extro/workflows"))?;

    // Create workspace Cargo.toml
    fs::write(
        root.join("Cargo.toml"),
        r#"[workspace]
members = ["crates/core", "crates/wasm", "crates/agent"]
resolver = "2"

[workspace.package]
edition = "2021"
license = "MIT"
version = "0.1.0"

[workspace.dependencies]
anyhow = "1"
clap = { version = "4.5", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
wasm-bindgen = "=0.2.114"
serde-wasm-bindgen = "0.6"
"#,
    )?;

    // Create extension manifest
    fs::write(
        root.join("extension/manifest.json"),
        r#"{
  "manifest_version": 3,
  "name": "Extro Extension",
  "version": "0.1.0",
  "description": "Built with Extro",
  "action": {
    "default_popup": "src/popup/index.html"
  },
  "background": {
    "service_worker": "src/background/index.js",
    "type": "module"
  },
  "permissions": ["storage", "activeTab", "scripting"],
  "host_permissions": ["<all_urls>"],
  "content_scripts": [
    {
      "matches": ["<all_urls>"],
      "js": ["src/content/index.js"],
      "run_at": "document_idle"
    }
  ]
}
"#,
    )?;

    // Create README
    fs::write(
        root.join("README.md"),
        format!(
            r#"# {name}

Built with [Extro](https://github.com/askpext/extro) - Rust-first browser extension framework.

## Quick Start

```bash
# Install dependencies
pnpm install

# Start development mode
pnpm dev

# Build for production
pnpm build
```

## Development

```bash
# Watch for changes
extro watch

# Launch browser with extension
extro dev-inject chrome

# Run tests
extro test
```

## Project Structure

```
{name}/
├── crates/
│   ├── core/       # Pure Rust domain logic
│   └── wasm/       # WASM bindings
├── extension/      # Browser extension shell
│   └── src/
│       ├── background/  # Service worker
│       ├── content/     # Content scripts
│       ├── popup/       # Popup UI
│       └── shared/      # Shared utilities
└── dist/           # Build output
```

## License

MIT
"#
        ),
    )?;

    // Create .gitignore
    fs::write(
        root.join(".gitignore"),
        r#"/target
/dist
/.extro
/node_modules
*.log
"#,
    )?;

    // Create package.json
    fs::write(
        root.join("extension/package.json"),
        r#"{
  "name": "extro-extension-runtime",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "extro watch",
    "build": "extro build",
    "test": "extro test"
  }
}
"#,
    )?;

    // Create Agentic Workflows
    fs::write(
        root.join(".extro/workflows/add-browser-effect.md"),
        r#"# Workflow: Adding a Browser Effect (Rust -> JS)

1.  **Define Effect**: Add a new variant to `BrowserEffect` enum in `crates/core/src/lib.rs`.
2.  **Dispatch Logic**: Update `CoreState::dispatch` to return the new effect.
3.  **Implement Handler**: Add a case for the new effect in `extension/src/background/index.js` inside `applyEffect`.
4.  **Verify**: Run `extro build` and test the interaction.
"#,
    )?;

    fs::write(
        root.join("ai.md"),
        format!(
            r#"# AI Agent Guide for {name}

Welcome, Agent. This project follows the **Extro Pattern**:

- **Domain Logic**: Lives in `crates/core/src/lib.rs`.
- **WASM Bridge**: Lives in `crates/wasm/src/lib.rs`.
- **Side Effects**: Managed in `extension/src/background/index.js`.
- **UI**: Fast React/Vanilla components in `extension/src/popup/`.

## Key Commands for Agents
- `extro assistant status`: Get project health.
- `extro assistant suggest`: Get next recommended steps.
- `extro build`: Compile WASM and package extension.
"#
        ),
    )?;

    // Create Core Crate
    fs::write(
        root.join("crates/core/Cargo.toml"),
        r#"[package]
name = "extro-core"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
serde = { workspace = true }
thiserror = { workspace = true }
"#,
    )?;

    fs::write(
        root.join("crates/core/src/lib.rs"),
        r#"use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RuntimeSurface { Background, ContentScript, Popup, Sidebar }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserSnapshot { pub url: String, pub title: String, pub selected_text: Option<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoreAction { SyncState }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreCommand { pub surface: RuntimeSurface, pub action: CoreAction, pub snapshot: BrowserSnapshot }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrowserEffect { ShowPopupToast { message: String } }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreResult { pub message: String, pub effects: Vec<BrowserEffect> }

#[derive(Debug, Default)]
pub struct CoreState { pub session_counter: u64 }

impl CoreState {
    pub fn new() -> Self { Self::default() }
    pub fn dispatch(&mut self, command: CoreCommand) -> CoreResult {
        self.session_counter += 1;
        match command.action {
            CoreAction::SyncState => CoreResult {
                message: format!("State synced (#{})", self.session_counter),
                effects: vec![],
            }
        }
    }
}
"#,
    )?;

    // Create WASM Crate
    fs::write(
        root.join("crates/wasm/Cargo.toml"),
        r#"[package]
name = "extro-wasm"
version.workspace = true
edition.workspace = true
license.workspace = true

[lib]
crate-type = ["cdylib"]

[dependencies]
extro-core = { path = "../core" }
wasm-bindgen = { workspace = true }
serde-wasm-bindgen = { workspace = true }
serde = { workspace = true }
"#,
    )?;

    fs::write(
        root.join("crates/wasm/src/lib.rs"),
        r#"use extro_core::{CoreCommand, CoreResult, CoreState};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct WasmEngine { inner: CoreState }

#[wasm_bindgen]
impl WasmEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self { Self { inner: CoreState::new() } }
    pub fn dispatch(&mut self, input: JsValue) -> Result<JsValue, JsValue> {
        let command: CoreCommand = serde_wasm_bindgen::from_value(input).map_err(|e| JsValue::from(e.to_string()))?;
        let result: CoreResult = self.inner.dispatch(command);
        serde_wasm_bindgen::to_value(&result).map_err(|e| JsValue::from(e.to_string()))
    }
}
"#,
    )?;

    // Create Extension Runtime
    fs::write(
        root.join("extension/src/background/index.js"),
        r#"import { runCore } from "../shared/engine.js";

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (message.type !== "extro.command") return false;
  runCore(message.payload).then(sendResponse);
  return true;
});
"#,
    )?;

    fs::write(
        root.join("extension/src/shared/engine.js"),
        r#"import init, { WasmEngine } from "../../pkg/extro_wasm.js";
let engine;
export async function getEngine() {
  if (engine) return engine;
  await init();
  engine = new WasmEngine();
  return engine;
}
export async function runCore(command) {
  const wasm = await getEngine();
  return wasm.dispatch(command);
}
"#,
    )?;

    fs::write(
        root.join("extension/src/popup/index.html"),
        r#"<!DOCTYPE html><html><head><link rel="stylesheet" href="styles.css"></head>
<body><main><h1>Extro</h1><button id="sync">Sync State</button><pre id="out"></pre></main>
<script type="module" src="index.js"></script></body></html>"#,
    )?;

    fs::write(
        root.join("extension/src/popup/index.js"),
        r#"document.getElementById('sync').onclick = async () => {
  const res = await chrome.runtime.sendMessage({
    type: "extro.command",
    payload: { surface: "Popup", action: "SyncState", snapshot: { url: location.href, title: document.title } }
  });
  document.getElementById('out').textContent = JSON.stringify(res, null, 2);
};"#,
    )?;

    // Create Agent Crate
    fs::write(
        root.join("crates/agent/Cargo.toml"),
        r#"[package]
name = "extro-agent"
version.workspace = true
edition.workspace = true
license.workspace = true

[dependencies]
extro-core = { path = "../core" }
serde = { workspace = true }
serde_json = { workspace = true }
"#,
    )?;

    fs::write(
        root.join("crates/agent/src/lib.rs"),
        r#"use extro_core::{CoreCommand, CoreResult, CoreState};
pub struct TraceableEngine { inner: CoreState }
impl TraceableEngine {
    pub fn new() -> Self { Self { inner: CoreState::new() } }
    pub fn dispatch(&mut self, command: CoreCommand) -> CoreResult { self.inner.dispatch(command) }
}
"#,
    )?;

    println!("✓ scaffolded {}", display_rel(&root));
    println!("\nNext steps:");
    println!("  cd {}", name);
    println!("  pnpm install");
    println!("  extro dev-inject chrome");

    Ok(())
}

fn display_rel(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn workspace_root() -> Result<PathBuf> {
    let mut current = env::current_dir().context("failed to resolve current directory")?;

    loop {
        if current.join("Cargo.toml").exists() && current.join("extension").exists() {
            return Ok(current);
        }

        if !current.pop() {
            bail!("could not locate Extro workspace root")
        }
    }
}

fn build_workspace(dev: bool) -> Result<()> {
    let root = workspace_root()?;
    let dist_dir = root.join("dist");
    let pkg_dir = dist_dir.join("pkg");
    let build_type = if dev { "debug" } else { "release" };
    let wasm_artifact = root.join(format!(
        "target/wasm32-unknown-unknown/{}/extro_wasm.wasm",
        build_type
    ));

    let build_args = if dev {
        vec![
            "build",
            "-p",
            "extro-wasm",
            "--target",
            "wasm32-unknown-unknown",
        ]
    } else {
        vec![
            "build",
            "-p",
            "extro-wasm",
            "--target",
            "wasm32-unknown-unknown",
            "--release",
        ]
    };

    run(
        ProcessCommand::new("cargo")
            .args(&build_args)
            .current_dir(&root),
        "cargo build for extro-wasm",
    )?;

    if dist_dir.exists() {
        fs::remove_dir_all(&dist_dir)
            .with_context(|| format!("failed to clean {}", dist_dir.display()))?;
    }

    fs::create_dir_all(&pkg_dir)?;

    run(
        ProcessCommand::new("wasm-bindgen")
            .arg("--target")
            .arg("web")
            .arg("--out-dir")
            .arg(&pkg_dir)
            .arg(&wasm_artifact)
            .current_dir(&root),
        "wasm-bindgen packaging",
    )?;

    copy_dir_all(
        &root.join("extension"),
        &dist_dir,
        &["package.json", "scripts"],
    )?;
    write_popup_style(&dist_dir.join("src/popup/styles.css"))?;

    let build_mode = if dev { "development" } else { "production" };
    println!(
        "✓ built unpacked extension at {} ({})",
        display_rel(&dist_dir),
        build_mode
    );
    Ok(())
}

fn watch_workspace() -> Result<()> {
    let root = workspace_root()?;
    let watch_roots = [
        root.join("crates"),
        root.join("cli"),
        root.join("extension"),
    ];
    let mut last = newest_mtime(&watch_roots)?;

    build_workspace(false)?;
    println!("watching for changes...");

    loop {
        thread::sleep(Duration::from_secs(2));
        let next = newest_mtime(&watch_roots)?;
        if next > last {
            println!("change detected, rebuilding...");
            if let Err(err) = build_workspace(false) {
                eprintln!("build failed: {err:#}");
            }
            last = next;
        }
    }
}

fn dev_inject(browser: &str) -> Result<()> {
    let root = workspace_root()?;
    let dist_dir = root.join("dist");

    if !dist_dir.exists() {
        build_workspace(false)?;
    }

    let browser_path = resolve_browser(browser)?;
    let user_data_dir = root.join(".extro/chrome-profile");
    fs::create_dir_all(&user_data_dir)?;

    ProcessCommand::new(browser_path)
        .arg(format!("--load-extension={}", dist_dir.display()))
        .arg(format!("--user-data-dir={}", user_data_dir.display()))
        .arg("--no-first-run")
        .arg("--disable-default-apps")
        .spawn()
        .context("failed to launch browser for dev inject")?;

    println!("launched {} with {}", browser, display_rel(&dist_dir));
    Ok(())
}

fn run_tests(package: &str) -> Result<()> {
    let root = workspace_root()?;

    println!("Running Extro tests...\n");

    match package {
        "core" => {
            println!("Testing extro-core...\n");
            run(
                ProcessCommand::new("cargo")
                    .arg("test")
                    .arg("-p")
                    .arg("extro-core")
                    .current_dir(&root),
                "cargo test for extro-core",
            )?;
        }
        "wasm" => {
            println!("Testing extro-wasm...\n");
            run(
                ProcessCommand::new("cargo")
                    .arg("test")
                    .arg("-p")
                    .arg("extro-wasm")
                    .current_dir(&root),
                "cargo test for extro-wasm",
            )?;
        }
        "all" | _ => {
            println!("Testing all packages...\n");
            run(
                ProcessCommand::new("cargo")
                    .arg("test")
                    .arg("--workspace")
                    .current_dir(&root),
                "cargo test for workspace",
            )?;
        }
    }

    println!("\n✓ All tests passed!");
    Ok(())
}

fn clean_workspace() -> Result<()> {
    let root = workspace_root()?;

    let dirs_to_clean = [
        root.join("target"),
        root.join("dist"),
        root.join(".extro"),
        root.join("pkg"),
    ];

    for dir in &dirs_to_clean {
        if dir.exists() {
            fs::remove_dir_all(dir)
                .with_context(|| format!("failed to clean {}", dir.display()))?;
            println!("✓ cleaned {}", display_rel(dir));
        }
    }

    println!("\nWorkspace cleaned!");
    Ok(())
}

fn package_extension(format: &str) -> Result<()> {
    let root = workspace_root()?;
    let dist_dir = root.join("dist");

    if !dist_dir.exists() {
        println!("Building extension first...");
        build_workspace(false)?;
    }

    match format {
        "zip" => {
            let zip_path = root.join("extro-extension.zip");

            #[cfg(windows)]
            {
                ProcessCommand::new("powershell")
                    .arg("-Command")
                    .arg("Compress-Archive")
                    .arg("-Path")
                    .arg(dist_dir.join("*"))
                    .arg("-DestinationPath")
                    .arg(&zip_path)
                    .arg("-Force")
                    .status()
                    .context("failed to create zip archive")?;
            }

            #[cfg(unix)]
            {
                ProcessCommand::new("zip")
                    .arg("-r")
                    .arg(&zip_path)
                    .arg(".")
                    .current_dir(&dist_dir)
                    .status()
                    .context("failed to create zip archive")?;
            }

            println!("✓ packaged extension to {}", display_rel(&zip_path));
        }
        "crx" => {
            println!("CRX packaging requires Chrome extension keys. Use ZIP for now.");
        }
        _ => {
            bail!("Unknown package format: {}. Use 'zip' or 'crx'.", format);
        }
    }

    Ok(())
}

fn show_info() -> Result<()> {
    let root = workspace_root().ok();

    println!("╔════════════════════════════════════════╗");
    println!("║         Extro Framework Info          ║");
    println!("╚════════════════════════════════════════╝\n");

    println!("Version: {}", env!("CARGO_PKG_VERSION"));

    // Get Rust version from rustc
    let rust_version = std::process::Command::new("rustc")
        .arg("--version")
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .unwrap_or_else(|| "unknown".to_string());
    println!("Rust: {}", rust_version.trim());

    if let Some(root) = &root {
        println!("\nWorkspace: {}", display_rel(root));

        let dist_dir = root.join("dist");
        if dist_dir.exists() {
            println!("✓ Build output exists");
        } else {
            println!("✗ No build output (run 'extro build')");
        }
    } else {
        println!("\n⚠ Not in an Extro workspace");
    }

    println!("\nCommands:");
    println!("  extro new <name>     Create new extension");
    println!("  extro build          Build extension");
    println!("  extro watch          Watch for changes");
    println!("  extro dev-inject     Launch browser with extension");
    println!("  extro test           Run tests");
    println!("  extro clean          Clean build artifacts");
    println!("  extro package        Package for distribution");
    println!("  extro info           Show this info");

    Ok(())
}

fn resolve_browser(browser: &str) -> Result<PathBuf> {
    let candidates = match browser {
        "chrome" | "chromium" => vec![
            PathBuf::from(r"C:\Program Files\Google\Chrome\Application\chrome.exe"),
            PathBuf::from(r"C:\Program Files (x86)\Google\Chrome\Application\chrome.exe"),
            PathBuf::from(r"C:\Program Files\Chromium\Application\chrome.exe"),
        ],
        "edge" => vec![
            PathBuf::from(r"C:\Program Files (x86)\Microsoft\Edge\Application\msedge.exe"),
            PathBuf::from(r"C:\Program Files\Microsoft\Edge\Application\msedge.exe"),
        ],
        other => vec![PathBuf::from(other)],
    };

    candidates
        .into_iter()
        .find(|path| path.exists())
        .with_context(|| format!("could not find a browser executable for {}", browser))
}

fn newest_mtime(paths: &[PathBuf]) -> Result<SystemTime> {
    let mut newest = SystemTime::UNIX_EPOCH;

    for path in paths {
        visit_mtime(path, &mut newest)?;
    }

    Ok(newest)
}

fn visit_mtime(path: &Path, newest: &mut SystemTime) -> Result<()> {
    let metadata = fs::metadata(path)?;
    let modified = metadata.modified()?;
    if modified > *newest {
        *newest = modified;
    }

    if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            visit_mtime(&entry.path(), newest)?;
        }
    }

    Ok(())
}

fn copy_dir_all(src: &Path, dst: &Path, ignored_names: &[&str]) -> Result<()> {
    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let entry_path = entry.path();
        let name = entry.file_name();
        let name_str = name.to_string_lossy();

        if ignored_names.iter().any(|ignored| *ignored == name_str) {
            continue;
        }

        let target_path = dst.join(&name);
        if entry.file_type()?.is_dir() {
            copy_dir_all(&entry_path, &target_path, ignored_names)?;
        } else {
            fs::copy(&entry_path, &target_path).with_context(|| {
                format!(
                    "failed to copy {} to {}",
                    entry_path.display(),
                    target_path.display()
                )
            })?;
        }
    }

    Ok(())
}

fn run(command: &mut ProcessCommand, label: &str) -> Result<()> {
    let status = command
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to start {label}"))?;

    if !status.success() {
        bail!("{label} failed with status {status}");
    }

    Ok(())
}

fn write_popup_style(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let mut file = fs::File::create(path)?;
    file.write_all(
        br#":root {
  color-scheme: light;
  font-family: "IBM Plex Sans", "Segoe UI", sans-serif;
  background:
    radial-gradient(circle at top left, #f3d9b1 0, transparent 40%),
    linear-gradient(160deg, #f8f4ec, #dfebf5);
  color: #1d2428;
}

body {
  min-width: 320px;
  margin: 0;
}

main {
  padding: 18px;
}

h1 {
  margin: 0 0 12px;
  font-size: 24px;
}

button {
  border: 0;
  border-radius: 999px;
  padding: 10px 14px;
  font-weight: 600;
  background: #1d6c61;
  color: #fff;
  cursor: pointer;
}

pre {
  margin-top: 14px;
  font-size: 12px;
  white-space: pre-wrap;
}
"#,
    )?;

    Ok(())
}
fn assistant_status(format: &str) -> Result<()> {
    let root = workspace_root().ok();

    if format == "json" {
        let status = serde_json::json!({
            "version": env!("CARGO_PKG_VERSION"),
            "in_workspace": root.is_some(),
            "workspace_path": root.as_ref().map(|r| display_rel(r)),
            "crates": ["core", "wasm"],
            "has_dist": root.as_ref().map(|r| r.join("dist").exists()).unwrap_or(false),
            "manifest_v3": root.as_ref().map(|r| r.join("extension/manifest.json").exists()).unwrap_or(false),
            "agent_ready": true,
            "next_steps": [
                "extro build",
                "extro test",
                "extro assistant suggest"
            ]
        });

        println!("{}", serde_json::to_string_pretty(&status)?);
    } else {
        println!("Extro Assistant is ready to help.");
        if let Some(r) = root {
            println!("You are in an Extro workspace at {}.", display_rel(&r));
        }
    }

    Ok(())
}

fn assistant_suggest() -> Result<()> {
    let root = workspace_root()?;

    println!("Suggestions for the Agent:");

    if !root.join("dist").exists() {
        println!("1. Run 'extro build' to generate the initial WASM package.");
    }

    if root.join("crates/core/src/lib.rs").exists() {
        println!("2. Add a new 'BrowserEffect' in 'crates/core/src/lib.rs' to extend bridge capabilities.");
    }

    println!("3. Run 'extro test' to verify the current state logic.");

    Ok(())
}
