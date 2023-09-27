use wasm_bindgen::JsValue;
use web_sys::window;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

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
pub fn resolve_path_files(path: &str) -> Vec<&str> {

  let components: Vec<&str> = path.split('/').collect();
  let mut resolved_components: Vec<&str> = Vec::new();

  for component in components.iter() {
    match component {
      &".." => {
        // If the component is '..', remove the last resolved component
        if !resolved_components.is_empty() {
          resolved_components.pop();
        }
      }
      &"." | &"" => {
        // If the component is '.' or empty, ignore
      }
      _ => {
        // Otherwise, add the component to the resolved path
        resolved_components.push(component);
      }
    }
  }

  resolved_components
}

pub fn resolve_path(path: &str) -> String {
  let mut out = resolve_path_files(path).join("/");
  if out.starts_with("/") {
    out.remove(0);
  }
  out
}
pub async fn fetch(url: String) -> Result<String, String> {
  let mut opts = RequestInit::new();
  opts.method("GET");
  opts.mode(RequestMode::Cors);

  let request = Request::new_with_str_and_init(&url, &opts).expect("failed to init request");

  let window = web_sys::window().unwrap();
  let resp_value = JsFuture::from(window.fetch_with_request(&request)).await.expect("request failed");

  // `resp_value` is a `Response` object.
  assert!(resp_value.is_instance_of::<Response>());
  let resp: Response = resp_value.dyn_into().unwrap();

  // Convert this other `Promise` into a rust `Future`.
  let text = JsFuture::from(resp.text().expect("failed to get text")).await.expect("failed to get text");

  // Send the JSON response back to JS.
  Ok(text.as_string().expect("failed to convert text"))
}


