import init, { WasmEngine, classifyUrl } from "../../pkg/extro_wasm.js";

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

export { classifyUrl };
