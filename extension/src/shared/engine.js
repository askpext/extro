import init, { WasmEngine, classifyUrl } from "../../pkg/extro_wasm.js";
import { ExtroEngine } from "../../../npm/runtime/src/index.js";

let engine;

export async function getEngine() {
  if (engine) return engine;
  const runner = new ExtroEngine({ WasmEngine });
  await runner.init(init);
  engine = runner.instance;
  return engine;
}

export async function runCore(command) {
  const wasm = await getEngine();
  return wasm.dispatch(command);
}

export { classifyUrl };
