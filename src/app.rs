use crate::termstate::TermState;

pub trait App {
  fn readchar(&mut self, state:&mut TermState, input: char) -> (Option<Box<dyn App>>, String);
}

pub struct EmptyApp{}

impl EmptyApp{
  pub fn new() -> Self{
    return Self{}
  }
}

impl App for EmptyApp{
  fn readchar(&mut self, _state: &mut TermState, _input: char) -> (Option<Box<dyn App>>, String) {
     panic!("NOT IMPLEMENTED");
  }
}