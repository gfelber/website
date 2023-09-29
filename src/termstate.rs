use crate::filesystem;

#[macro_export]
macro_rules! clear {
    ($state:expr) => {{
      $state.cursor_y = 0;
      $state.cursor_x = 0;
      let cleared: String = "\n".repeat($state.height);
      let ups: String = consts::UP.repeat($state.height);
      write!("{}{}\r", cleared, &ups);
    }};
}

#[macro_export]
macro_rules! clearln {
    ($state:expr) => {{
      state.cursor_x = 0;
      let out: String = consts::RETURN.repeat($state.cursor_x);
      write!("{}", out);
    }};
}

pub struct TermState {
  pub path: &'static filesystem::Entry,
  pub cursor_x: usize,
  pub cursor_y: usize,
  pub height: usize,
  pub width: usize,
}

impl TermState {
  pub fn new() -> Self {
    Self {
      path: &filesystem::ROOT,
      cursor_x: 0,
      cursor_y: 0,
      height: 0,
      width: 0,
    }
  }
}
