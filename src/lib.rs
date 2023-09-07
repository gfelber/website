#![feature(str_split_remainder)]
mod utils;

use include_dir::{include_dir, Dir};
use wasm_bindgen::prelude::*;
use lazy_static::lazy_static;
use std::sync::{Mutex, MutexGuard};

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_u32(a: u32);

    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_char(a: Option<char>);
}

static ROOT: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/root");

static mut WIDTH: usize = 0;
static mut HEIGHT: usize = 0;
const PREFIX: &str = "$ " ;
#[wasm_bindgen]
pub fn init(height: usize, width: usize) -> String{
    log(ROOT.get_file("test_file").unwrap().contents_utf8().unwrap());
    log(ROOT.get_dir("test_dir").unwrap().path().to_str().unwrap());
    unsafe {
      WIDTH = width - PREFIX.len(); // remove because of shell
      HEIGHT = height;
    }
    return PREFIX.to_string();
}

#[wasm_bindgen]
pub fn readline(input: &str) -> String {
    let mut vec = Vec::<String>::new(); 
    for c in  input.chars(){
      vec.push(readchar(c));
    }
    return vec.join("");
}

lazy_static! {
    static ref INPUT_BUFFER: Mutex<Vec<char>> = Mutex::new(Vec::new());
}
lazy_static! {
    static ref ESCAPE_BUFFER: Mutex<Vec<char>> = Mutex::new(Vec::new());
}
lazy_static! {
    static ref HISTORY: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
}

static mut HISTORY_INDEX: usize = 0;
static mut ESCAPE: bool = false;
static mut CURSOR_X: usize = 0;
static mut CURSOR_Y: usize = 0;

const UP: &str = "\x1b\x5b\x41";
const DOWN: &str = "\x1b\x5b\x42";
const RIGHT: &str = "\x1b\x5b\x43";
const LEFT: &str = "\x1b\x5b\x44";
const RETURN: &str = "\x1b\x5b\x44 \x1b\x5b\x44";
const NEWLINE: &str = "\n\r";

pub fn clear() -> String {
  unsafe{ CURSOR_Y = 0 };
  unsafe{ CURSOR_X = 0 };
  let cleared: String = "\n".repeat(unsafe{ HEIGHT });
  let ups: String = UP.repeat(unsafe{ HEIGHT });
  return cleared + &ups + "\r" + PREFIX;
}

pub fn clearline() -> String {
  let out:String = RETURN.repeat(unsafe{ CURSOR_X });
  unsafe{ CURSOR_X = 0 };
  return out;
}

pub fn echo(input: &str) -> String {
    unsafe {CURSOR_Y += 1};
    return NEWLINE.to_string() + input + NEWLINE + PREFIX;
}

pub fn command(cmdline: &str) -> String {
  let mut history = HISTORY.lock().unwrap();
  history.push(Box::leak(cmdline.to_owned().into_boxed_str()));
  unsafe{HISTORY_INDEX = history.len()};
  let mut cmd_args = cmdline.split(" ");
  let cmdline = cmd_args.next().unwrap();
  unsafe{ CURSOR_Y += 1 };
  match cmdline {
    "clear" => return clear(),
    "echo" => return echo(cmd_args.remainder().unwrap()),
    _ => {
      return NEWLINE.to_string() + PREFIX
    }
  }
}

pub fn escape(escapestr: &str, input_buffer: &mut MutexGuard<'_, Vec<char>>) -> String {
  let history = HISTORY.lock().unwrap();
  match escapestr {
    UP => {
      if unsafe{HISTORY_INDEX > 0} {
        unsafe{HISTORY_INDEX -= 1}
        let entry = history[unsafe{HISTORY_INDEX}]; 
        input_buffer.clear();
        input_buffer.extend(entry.chars());
        let out = clearline() + entry;
        unsafe{CURSOR_X = entry.len()};
        return out;
      }
      return "".to_string();
    },
    DOWN => {
      if history.len() != 0 && unsafe{HISTORY_INDEX < history.len() - 1}{
        unsafe{HISTORY_INDEX += 1}
        let entry = history[unsafe{HISTORY_INDEX}]; 
        input_buffer.clear();
        input_buffer.extend(entry.chars());
        let out = clearline() + entry;
        unsafe{CURSOR_X = entry.len()};
        return out;
      }
      return "".to_string();
    },
    RIGHT => {
      if unsafe{CURSOR_X < input_buffer.len()} {
        unsafe{CURSOR_X += 1}
        return RIGHT.to_string();
      }
    },
    LEFT => {
      if unsafe{CURSOR_X >= PREFIX.len()} {
        unsafe{CURSOR_X -= 1}
        return LEFT.to_string();
      }
    },
    _ => {},
  }
  return "".to_string()
}

#[wasm_bindgen]
pub fn readchar(input: char) -> String {
    log(&format!("{:02x}", input as u32));
    let mut escape_buffer = ESCAPE_BUFFER.lock().unwrap();
    let mut input_buffer = INPUT_BUFFER.lock().unwrap();
    if unsafe{ESCAPE} {
      escape_buffer.push(input);
      let escapestr: String = escape_buffer.iter().collect();
      let out: String = escape(&escapestr, &mut input_buffer);
      if out != "" || escape_buffer.len() == 3 {
        unsafe { ESCAPE=false };
        escape_buffer.clear(); 
      }
      return out;
    }
    match input {
      '\r'|'\n' => {
          let cmd: String = input_buffer.iter().collect();
          log(&cmd);
          input_buffer.clear();
          unsafe{ CURSOR_X = 0 };
          return command(&cmd);
      },
      // clear line
      '\x15' => {
        input_buffer.clear();
        return clearline();
      },
      // clear
      '\x0c' => {
        input_buffer.clear();
        return clear();
      },
      // return key
      '\x7f' => {
          if input_buffer.is_empty() {
            return "".to_string();
          }
          input_buffer.pop();
          unsafe{ CURSOR_X -= 1 };
          return RETURN.to_string();
      },
      // escape
      '\x1b' => {
        unsafe { ESCAPE = true };
        escape_buffer.push(input);
        return "".to_string();
      },
      _ => {
        if unsafe{ CURSOR_X < input_buffer.len() } {
          unsafe{ input_buffer[CURSOR_X] = input }
        } else if unsafe{ CURSOR_X < WIDTH} {
          unsafe{ CURSOR_X += 1 };
          input_buffer.push(input);
        } else {
          log("reached EOL");
        }
        return input.to_string();
      }
    }
}
