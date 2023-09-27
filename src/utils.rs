use wasm_bindgen::JsValue;
use web_sys::{window, XmlHttpRequest};

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
pub fn fetch(url: String) -> Result<String, String> {
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


