/**
 * Extro Runtime - The JavaScript adapter for the Extro browser extension framework.
 */

export class ExtroEngine {
  constructor(wasm) {
    this.wasm = wasm;
    this.instance = null;
  }

  /**
   * Initialize the engine with the provided WASM init function.
   * @param {Function} initFn - The init function from the generated WASM bindings.
   */
  async init(initFn) {
    if (this.instance) return this.instance;
    await initFn();
    this.instance = new this.wasm.WasmEngine();
    return this.instance;
  }

  /**
   * Dispatch a command to the Rust core.
   * @param {Object} command - The command object to be processed.
   */
  async dispatch(command) {
    if (!this.instance) {
      throw new Error("ExtroEngine not initialized. Call init() first.");
    }
    return this.instance.dispatch(command);
  }

  /**
   * Get telemetry snapshots from the Rust core.
   */
  async telemetry() {
    if (!this.instance) {
      throw new Error("ExtroEngine not initialized. Call init() first.");
    }
    return this.instance.telemetry();
  }
}

/**
 * Utility for easy engine creation.
 */
export async function createEngine(wasmLoader) {
  const wasm = await wasmLoader();
  const engine = new ExtroEngine(wasm);
  await engine.init(wasm.default);
  return engine;
}

export { logger, createTimer, withErrorLog } from "./logger.js";
