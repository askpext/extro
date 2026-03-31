use extro_core::{CoreCommand, CoreResult, CoreState};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;

#[wasm_bindgen]
pub struct WasmEngine {
    inner: CoreState,
}

#[wasm_bindgen]
impl WasmEngine {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: CoreState::new(),
        }
    }

    #[wasm_bindgen(js_name = dispatch)]
    pub fn dispatch(&mut self, input: JsValue) -> Result<JsValue, JsValue> {
        let command: CoreCommand =
            serde_wasm_bindgen::from_value(input).map_err(|err| JsValue::from(err.to_string()))?;
        let result: CoreResult = self.inner.dispatch(command);
        serde_wasm_bindgen::to_value(&result).map_err(|err| JsValue::from(err.to_string()))
    }

    #[wasm_bindgen(js_name = telemetry)]
    pub fn telemetry(&self) -> Result<JsValue, JsValue> {
        serde_wasm_bindgen::to_value(&self.inner.telemetry())
            .map_err(|err| JsValue::from(err.to_string()))
    }
}

#[wasm_bindgen(js_name = classifyUrl)]
pub fn classify_url(url: &str) -> String {
    if url.contains("github.com") {
        "developer".into()
    } else if url.contains("docs") {
        "documentation".into()
    } else {
        "general".into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_url_github() {
        assert_eq!(classify_url("https://github.com/rust-lang/rust"), "developer");
    }

    #[test]
    fn test_classify_url_docs() {
        assert_eq!(classify_url("https://docs.example.com/guide"), "documentation");
    }

    #[test]
    fn test_classify_url_general() {
        assert_eq!(classify_url("https://example.com/page"), "general");
    }

    // Note: WASM engine tests require wasm-bindgen-test runner
    // Run with: wasm-pack test --headless --firefox
}

