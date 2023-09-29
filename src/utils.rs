use std::ptr::eq;
use wasm_bindgen::JsValue;
use web_sys::{window, XmlHttpRequest};
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::{Request, RequestInit, RequestMode, Response};

#[wasm_bindgen(module="/src/js/dist/package.js")]
#[no_mangle]
extern "C" {
  fn write(out: String);

  fn create_term(options: JsValue) -> JsValue;
}

pub fn term_write(out_str: impl Into<String>) {
  let out = out_str.into();
  write(out);
}

#[wasm_bindgen]
pub unsafe fn term(options: JsValue) -> JsValue{
  return create_term(options);
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

pub fn change_url(new_url_str: impl Into<String>) -> Result<(), JsValue> {
  let new_url = new_url_str.into();
  // Get a reference to the window's history object
  let window = window().expect("Should have a window in this context");
  let history = window.history().expect("Should have a history object in this context");

  // Push the new URL onto the history stack without reloading the page
  history.push_state_with_url(&JsValue::NULL, "", Some(&new_url))
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

pub fn resolve_path(path_str: impl Into<String>) -> String {
  let path = path_str.into();
  let mut out = resolve_path_files(&path).join("/");
  if out.starts_with("/") {
    out.remove(0);
  }
  out
}

#[deprecated]
pub fn fetch(url_str: impl Into<String>) -> Result<String, String> {
  let url = url_str.into();
  // Create a new XMLHttpRequest to fetch the file
  let xhr = XmlHttpRequest::new().expect("failed to create XmlRttpRequest");
  xhr.open_with_async(&"GET", &url, false).expect("failed to open");

  // Send the request synchronously
  xhr.send().expect("failed to send request");

  // Check if the request was successful (status code 200)
  if xhr.status().expect("failed to get status") != 200 {
    return Err("HTTP request failed".to_string());
  }

  // Convert the response to a Vec<u8>
  let response = xhr.response_text().unwrap().unwrap();

  Ok(response)
}

pub async fn afetch(url_str: impl Into<String>) -> Result<String, String> {
  let url = url_str.into();
  let mut opts = RequestInit::new();
  opts.method("GET");
  opts.mode(RequestMode::Cors);

  let request = Request::new_with_str_and_init(&url, &opts).expect("failed to create request");

  let window = web_sys::window().unwrap();
  let resp_value = JsFuture::from(window.fetch_with_request(&request)).await.expect("request failed");

  // `resp_value` is a `Response` object.
  assert!(resp_value.is_instance_of::<Response>());
  let resp: Response = resp_value.dyn_into().unwrap();

  // Convert this other `Promise` into a rust `Future`.
  let text = JsFuture::from(
    resp.text().expect("failed to get text promise")
  ).await.expect("failed to get text from promise");

  // Send the JSON response back to JS.
  Ok(text.as_string().expect("failed to get text"))
}

pub fn human_size(size: u64) -> String {
  const KB: u64 = 1024;
  const MB: u64 = KB * 1024;
  const GB: u64 = MB * 1024;

  if size < KB {
    format!("{}", size)
  } else if size < MB {
    format!("{:.1}K", size as f64 / KB as f64)
  } else {
    format!("{:.1}G", size as f64 / GB as f64)
  }
}


