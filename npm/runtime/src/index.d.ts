/**
 * @extro/runtime — TypeScript type definitions
 *
 * The JavaScript runtime adapter for the Extro browser extension framework.
 */

/**
 * A WASM module containing the WasmEngine constructor.
 */
export interface WasmModule {
  WasmEngine: new () => WasmEngineInstance;
}

/**
 * An instance of the WASM engine with dispatch and telemetry methods.
 */
export interface WasmEngineInstance {
  dispatch(command: CoreCommand): CoreResult;
  telemetry(): string[];
}

/**
 * The surface that originated a command.
 */
export type RuntimeSurface = "Background" | "ContentScript" | "Popup" | "Sidebar";

/**
 * Actions that can be dispatched to the Rust core.
 */
export type CoreAction = "AnalyzeSelection" | "SummarizePage" | "SyncState" | string;

/**
 * A snapshot of the current browser state.
 */
export interface BrowserSnapshot {
  url: string;
  title: string;
  selected_text: string | null;
}

/**
 * A command sent from JavaScript to the Rust core.
 */
export interface CoreCommand {
  surface: RuntimeSurface;
  action: CoreAction;
  snapshot: BrowserSnapshot;
}

/**
 * A browser side effect requested by the Rust core.
 */
export type BrowserEffect =
  | { ReadDomSelection: Record<string, never> }
  | { ReadClipboard: Record<string, never> }
  | { PersistSession: { key: string; value: string } }
  | { ShowPopupToast: { message: string } }
  | { OpenSidePanel: { route: string } }
  | { InjectContentScript: { file: string } };

/**
 * The result of processing a CoreCommand.
 */
export interface CoreResult {
  message: string;
  effects: BrowserEffect[];
}

/**
 * The main Extro engine class.
 * Wraps the WASM engine with initialization and dispatch methods.
 */
export declare class ExtroEngine {
  wasm: WasmModule;
  instance: WasmEngineInstance | null;

  constructor(wasm: WasmModule);

  /**
   * Initialize the engine with the provided WASM init function.
   * @param initFn - The init function from the generated WASM bindings.
   */
  init(initFn: () => Promise<void>): Promise<WasmEngineInstance>;

  /**
   * Dispatch a command to the Rust core.
   * @param command - The command object to be processed.
   * @throws Error if engine is not initialized.
   */
  dispatch(command: CoreCommand): Promise<CoreResult>;

  /**
   * Get telemetry snapshots from the Rust core.
   * @throws Error if engine is not initialized.
   */
  telemetry(): Promise<string[]>;
}

/**
 * Utility for easy engine creation.
 * @param wasmLoader - A function that returns the WASM module.
 */
export declare function createEngine(
  wasmLoader: () => Promise<WasmModule & { default: () => Promise<void> }>
): Promise<ExtroEngine>;

/**
 * Log levels for the structured logger.
 */
export type LogLevel = "info" | "warn" | "error" | "debug";

/**
 * Structured logger for Extro extensions.
 */
export declare const logger: {
  info(message: string, data?: unknown): void;
  warn(message: string, data?: unknown): void;
  error(message: string, error?: unknown): void;
  debug(message: string, data?: unknown): void;
  command(command: CoreCommand, surface: string, result?: CoreResult): void;
};

/**
 * Create a performance timer for measuring operation duration.
 * @param label - Label for the timer.
 */
export declare function createTimer(label: string): {
  end(message: string): number;
};

/**
 * Wrap an async function with automatic error logging.
 * @param fn - The async function to wrap.
 * @param context - A label for error messages.
 */
export declare function withErrorLog<T extends (...args: unknown[]) => Promise<unknown>>(
  fn: T,
  context: string
): T;
