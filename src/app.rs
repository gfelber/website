use crate::termstate::TermState;

pub trait App: Send + Sync {
  fn readchar(&mut self, state: &mut TermState, input: char) -> Option<Box<dyn App>>;
  fn scroll(&mut self, _state: &mut TermState, _lines: i32) {}
  fn autocomplete(&self, state: &TermState) -> Vec<String>;
}

pub struct EmptyApp {}

impl App for EmptyApp {
  fn readchar(&mut self, _state: &mut TermState, _input: char) -> Option<Box<dyn App>> {
    panic!("NOT IMPLEMENTED");
  }
  fn autocomplete(&self, _state: &TermState) -> Vec<String> {
    panic!("NOT IMPLEMENTED");
  }
  fn scroll(&mut self, _state: &mut TermState, _lines: i32) {
    panic!("NOT IMPLEMENTED");
  }
}

impl EmptyApp {
  pub fn new() -> Self {
    Self {}
  }
}
