use wasm_bindgen::prelude::*;

fn err_to_js_value(err: impl std::error::Error) -> JsValue {
    JsValue::from_str(&err.to_string())
}

#[wasm_bindgen(js_name = "readTVG")]
pub fn read_tvg(data: &[u8]) -> Result<Vec<u8>, JsValue> {
    let data = tvg::read::read(&mut std::io::Cursor::new(data)).map_err(err_to_js_value)?;
    rmp_serde::to_vec_named(&data).map_err(err_to_js_value)
}
