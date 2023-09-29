use crate::consts;
use crate::filesystem;

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
  pub fn clear(&mut self) -> String {
    self.cursor_y = 0;
    self.cursor_x = 0;
    let cleared: String = "\n".repeat(self.height);
    let ups: String = consts::UP.repeat(self.height);
    return cleared + &ups + "\r";
  }
}
