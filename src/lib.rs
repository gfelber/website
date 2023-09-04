mod utils;

use wasm_bindgen::prelude::*;
use std::sync::Mutex;
use lazy_static::lazy_static;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_u32(a: u32);

    #[wasm_bindgen(js_namespace = console, js_name = log)]
    fn log_char(a: Option<char>);
}

static mut WIDTH: usize = 0;
static mut HEIGHT: usize = 0;
#[wasm_bindgen]
pub fn init(height: usize, width: usize){
    unsafe {
      WIDTH = width - 2; // remove because of shell
      HEIGHT = height;
    }
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
static mut ESCAPE: u8 = 0;
static mut CURSOR_X: usize = 0;
static mut CURSOR_Y: usize = 0;
const UP: &str = "\x1b\x5b\x41";
const DOWN: &str = "\x1b\x5b\x42";
const RIGHT: &str = "\x1b\x5b\x43";
const LEFT: &str = "\x1b\x5b\x44";
const RETURN: &str = "\x1b\x5b\x44 \x1b\x5b\x44";

#[wasm_bindgen]
pub fn readchar(input: char) -> String {
    log(&format!("{:02x}", input as u32));
    if unsafe{ESCAPE != 0} {
      unsafe { ESCAPE -= 1 };
      return input.to_string();
    }
    let mut input_buffer = INPUT_BUFFER.lock().unwrap();
    match input {
      '\r'|'\n'=> {
          let s: String = input_buffer.iter().collect::<String>();
          log(&s);
          input_buffer.clear();
          unsafe{ CURSOR_X = 0 };
          command(&s)
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
        unsafe { ESCAPE = 2 };
        return input.to_string();
      },
      _ => {
        unsafe{ CURSOR_X += 1 };
        if unsafe{ CURSOR_X < input_buffer.len() } {
          unsafe{ input_buffer[CURSOR_X] = input }
        } else if unsafe{ CURSOR_X < WIDTH} {
          input_buffer.push(input);
        } else {
          log("reached EOL");
          return "".to_string();
        }
        return input.to_string();
      }
    }
}

pub fn command(cmd: &str) -> String {
  match cmd {
    "clear" => return clear(),
    _ => return "\r\n$ ".to_string()
  }
}

pub fn clear() -> String {
  unsafe{ CURSOR_Y += HEIGHT - (CURSOR_Y % HEIGHT) };
  let cleared: String = "\n".repeat(unsafe{ HEIGHT });
  let ups: String = UP.repeat(unsafe{ HEIGHT });
  unsafe{ CURSOR_X = 0 };
  return cleared + &ups + "\r$ ";
}
