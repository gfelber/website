#![feature(str_split_remainder)]
#![feature(negative_impls)]


use std::ops::DerefMut;
use std::sync::Mutex;

use cfg_if::cfg_if;
use lazy_static::lazy_static;
use log::info;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::spawn_local;

mod utils;
mod consts;
mod app;
mod shell;
mod less;
mod termstate;
mod filesystem;

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
  info!("init");
  info!("{:?}", filesystem::ROOT.get_file("test_dir/test_dir2/test"));
  info!("{:?}", filesystem::ROOT.get_file("asdf"));
  info!("{:?}", filesystem::ROOT.get_file("test_dir/test_dir2/test").unwrap().load().unwrap());
  spawn_local(async {
    info!("{:?}", utils::afetch("/root/test_dir/test_dir2/test".to_string()).await.unwrap());
  });
  info!("done");
  term.init(height, width, location);
}

#[wasm_bindgen]
pub fn readline(input: &str) {
  let mut term = TERM.lock().unwrap();
  term.readline(input);
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
        self.state.path = &mut filesystem::ROOT.get_file(&(location_str.clone() + "/..")).unwrap();
        let mut less_app = less::Less::new();
        less_app.less(&mut self.state, &location_str).unwrap();
        self.app = Box::new(less_app);
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

  fn readchar(&mut self, input: char) {
    let x = self.state.cursor_x;
    let y = self.state.cursor_y;
    info!("{:02x}", input as u32);
    let app = self.app.readchar(self.state.deref_mut(), input);
    if app.is_some() {
      self.app = app.unwrap();
    }
    info!("({}|{})->({}|{})", x, y, self.state.cursor_x, self.state.cursor_y);
  }
}

