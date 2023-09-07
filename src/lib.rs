mod utils;

use include_dir::{include_dir, Dir};
use wasm_bindgen::prelude::*;
use lazy_static::lazy_static;
use std::sync::Mutex;

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
    log(ROOT.get_file("test").unwrap().contents_utf8().unwrap());
    unsafe {
      WIDTH = width - PREFIX.len(); // remove because of shell
      HEIGHT = height;
    }
    return PREFIX.to_string();
}

#[wasm_bindgen]
pub fn echo(input: char) -> String {
    return input.to_string();
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
static mut ESCAPE: bool = false;
static mut CURSOR_X: usize = 0;
static mut CURSOR_Y: usize = 0;

static mut IS_ARROW: bool = false;
const UP: &str = "\x1b\x5b\x41";
const DOWN: &str = "\x1b\x5b\x42";
const RIGHT: &str = "\x1b\x5b\x43";
const LEFT: &str = "\x1b\x5b\x44";
const RETURN: &str = "\x1b\x5b\x44 \x1b\x5b\x44";

pub fn clear() -> String {
  unsafe{ CURSOR_Y += HEIGHT - (CURSOR_Y % HEIGHT) };
  let cleared: String = "\n".repeat(unsafe{ HEIGHT });
  let ups: String = UP.repeat(unsafe{ HEIGHT });
  unsafe{ CURSOR_X = 0 };
  return cleared + &ups + "\r$ ";
}

pub fn command(cmd: &str) -> String {
  match cmd {
    "clear" => return clear(),
    _ => {
      unsafe {CURSOR_Y += 1};
      return "\r\n".to_string() + PREFIX
    }
  }
}

pub fn escape(escapestr: &str, len: usize) -> String {
  match escapestr {
    UP => {
      // TODO: implement command history
      return "".to_string();
    },
    DOWN => {
      // TODO: implement command history
      return "".to_string();
    },
    RIGHT => {
      if unsafe{CURSOR_X < len} {
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
      let out: String = escape(&escapestr, input_buffer.len());
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
        let out:String = RETURN.repeat(unsafe{ CURSOR_X });
        input_buffer.clear();
        unsafe{ CURSOR_X = 0 };
        return out;
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
    return input.to_string();
}
