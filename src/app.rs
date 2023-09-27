use crate::termstate::TermState;

pub trait App: Send + Sync {
  fn readchar(&mut self, state: &mut TermState, input: char) -> (Option<Box<dyn App>>, String);
}

pub struct EmptyApp {}

impl App for EmptyApp {
  fn readchar(&mut self, _state: &mut TermState, _input: char) -> (Option<Box<dyn App>>, String) {
    panic!("NOT IMPLEMENTED");
  }
}

impl EmptyApp {
  pub fn new() -> Self {
    Self{}
  }
}
