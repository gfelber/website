use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;
use web_sys::window;

#[wasm_bindgen]
extern "C" {
  #[wasm_bindgen(js_namespace = console)]
  pub fn log(s: &str);
}

pub fn set_panic_hook() {
  // When the `console_error_panic_hook` feature is enabled, we can call the
  // `set_panic_hook` function at least once during initialization, and then
  // we will get better error messages if our code ever panics.
  //
  // For more details see
  // https://github.com/rustwasm/console_error_panic_hook#readme
  #[cfg(feature = "console_error_panic_hook")]
  console_error_panic_hook::set_once();
}

pub fn change_url(new_url: &str) -> Result<(), JsValue> {
  // Get a reference to the window's history object
  let window = window().expect("Should have a window in this context");
  let history = window.history().expect("Should have a history object in this context");

  // Push the new URL onto the history stack without reloading the page
  history.push_state_with_url(&JsValue::NULL, "", Some(new_url))
    .map_err(|err| err.into())
}

pub fn resolve_path(path: &str) -> String {
  let components: Vec<&str> = path.split('/').collect();
  let mut resolved_components: Vec<&str> = Vec::new();

  for component in components.iter() {
    if component == &".." {
      // If the component is '..', remove the last resolved component
      if !resolved_components.is_empty() {
        resolved_components.pop();
      }
    } else {
      // Otherwise, add the component to the resolved path
      resolved_components.push(component);
    }
  }

  let mut out = resolved_components.join("/");
  if out.starts_with("/") {
    out.remove(0);
  }
  return out;
}

