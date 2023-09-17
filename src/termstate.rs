use include_dir::Dir;
use crate::consts;

pub struct TermState {
  pub path: &'static Dir<'static>,
  pub cursor_x: usize,
  pub cursor_y: usize,
  pub height: usize,
  pub width: usize,
}

impl TermState {
  pub fn new() -> Self{
    return Self{
      path: &consts::ROOT,
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
