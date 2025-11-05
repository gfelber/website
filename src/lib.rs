#![feature(str_split_remainder)]
#![feature(negative_impls)]

use std::ops::DerefMut;
use std::sync::Mutex;

use cfg_if::cfg_if;
use cmds::cmds_init;
use lazy_static::lazy_static;
use log::info;
use wasm_bindgen::prelude::*;

mod app;
mod cmds;
mod consts;
mod filesystem;
mod less;
mod shell;
mod termstate;
mod utils;

cfg_if! {
    if #[cfg(feature = "console_log")] {
        fn init_log() {
            use log::Level;
            console_log::init_with_level(Level::Trace).expect("error initializing log");
        }
    } else {
        fn init_log() {}
    }
}

lazy_static! {
  static ref TERM: Mutex<Term> = Mutex::new(Term::new());
}

#[wasm_bindgen]
pub fn init(height: usize, width: usize, location: &str) {
  let mut term = TERM.lock().unwrap();
  #[cfg(debug_assertions)]
  {
    if !term.init {
      init_log();
      utils::set_panic_hook();
    }
  }
  if !term.init {
    cmds_init();
  }
  info!("init");
  term.init(height, width, location);
}

#[wasm_bindgen]
pub fn readline(input: &str) {
  let mut term = TERM.lock().unwrap();
  term.readline(input);
}

#[wasm_bindgen]
pub fn scroll(lines: i32) {
  let mut term = TERM.lock().unwrap();
  term.scroll(lines);
}

#[wasm_bindgen]
pub fn autocomplete() -> Vec<JsValue> {
  let mut completions: Vec<JsValue> = Vec::new();
  let mut term = TERM.lock().unwrap();
  for completion in term.autocomplete() {
    completions.push(JsValue::from_str(&completion));
  }
  completions
}

pub struct Term {
  app: Box<dyn app::App>,
  state: Box<termstate::TermState>,
  init: bool,
}

impl Term {
  pub fn new() -> Self {
    Self {
      app: Box::new(app::EmptyApp::new()),
      state: Box::new(termstate::TermState::new()),
      init: false,
    }
  }

  pub fn init(&mut self, height: usize, width: usize, location: &str) {
    self.init = true;
    let mut location_str = location.to_string();
    location_str.remove(0);
    let path = filesystem::ROOT.get_file(&location_str.clone());
    self.state.width = width;
    self.state.height = height;
    if path.is_ok() {
      if path.clone().unwrap().is_dir {
        self.state.path = path.unwrap();
      } else {
        self.state.path = &mut filesystem::ROOT
          .get_file(&(location_str.clone() + "/.."))
          .unwrap();
        let mut less_app = less::Less::new();
        let offset = match location_str.rfind("/") {
          Some(off) => off + 1,
          None => 0,
        };
        let filename = &location_str[offset..];
        less_app.less(&mut self.state, filename).unwrap();
        self.app = Box::new(less_app);
        return;
      }
    }
    self.app = Box::new(shell::Shell::new());
    shell::Shell::clear(&mut self.state);
  }

  pub fn readline(&mut self, input: &str) {
    for c in input.chars() {
      self.readchar(c);
    }
  }

  pub fn autocomplete(&mut self) -> Vec<String> {
    self.app.autocomplete(&self.state)
  }

  pub fn scroll(&mut self, lines: i32) {
    info!("scroll {}", lines);
    self.app.scroll(self.state.deref_mut(), lines);
  }

  fn readchar(&mut self, input: char) {
    let x = self.state.cursor_x;
    let y = self.state.cursor_y;
    info!("input: {:02x}", input as u32);
    let app = self.app.readchar(self.state.deref_mut(), input);
    if app.is_some() {
      self.app = app.unwrap();
    }
    info!(
      "({}|{})->({}|{})",
      x, y, self.state.cursor_x, self.state.cursor_y
    );
  }
}
