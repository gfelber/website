#![feature(str_split_remainder)]
mod utils;

use include_dir::{include_dir, Dir, File};
use std::sync::{Mutex, MutexGuard};
use wasm_bindgen::prelude::*;
use lazy_static::lazy_static;
use wasm_bindgen::JsValue;
use web_sys::History;
use web_sys::window;
use std::str::Lines;


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
static mut PATH: &Dir<'_> = &ROOT;
static mut LESS_LINES:Vec<&str> = Vec::new();

static mut WIDTH: usize = 0;
static mut HEIGHT: usize = 0;
static mut LESS: bool = false;
const PREFIX: &str = "$ " ;
#[wasm_bindgen]
pub fn init(height: usize, width: usize, location: &str) -> String{
    log(location);
    log(ROOT.get_file("test_file").unwrap().contents_utf8().unwrap());
    log(ROOT.get_dir("test_dir/test_dir2").unwrap().path().to_str().unwrap());
    let mut location_str = location.to_string();
    location_str.remove(0);
    let path = ROOT.get_entry(location_str);
    if !path.is_none(){
      if path.unwrap().as_dir().is_none() {
        log("found file");
      } else {
        unsafe{PATH = path.unwrap().as_dir().unwrap()};
      }
    }
    unsafe {
      WIDTH = width - PREFIX.len(); // remove because of PREFIX
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
    static ref ANSI_BUFFER: Mutex<Vec<char>> = Mutex::new(Vec::new());
}
lazy_static! {
    static ref HISTORY: Mutex<Vec<&'static str>> = Mutex::new(Vec::new());
}

static mut HISTORY_INDEX: usize = 0;
static mut CURSOR_X: usize = 0;
static mut CURSOR_Y: usize = 0;

// ANSI
const UP: &str = "\x1b\x5b\x41";
const DOWN: &str = "\x1b\x5b\x42";
const RIGHT: &str = "\x1b\x5b\x43";
const LEFT: &str = "\x1b\x5b\x44";
const PAGE_DOWN: &str = "\x1b\x5b\x36\x7e";
const PAGE_UP: &str = "\x1b\x5b\x35\x7e";
const PAGE_START: &str = "\x1b\x5b\x48";
const PAGE_END: &str = "\x1b\x5b\x46";
const RETURN: &str = "\x1b\x5b\x44 \x1b\x5b\x44";
const NEWLINE: &str = "\n\r";

static mut ANSI: bool = false;
const HEADER_PURPLE: &str = "\x1b[95m";
const BLUE: &str = "\x1b[34m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const BLACK: &str = "\x1b[30m";
const BOLD: &str = "\x1b[1m";
const UNDERLINE: &str = "\x1b[4m";
const WHITE_BACKGROUND: &str = "\x1b[48;2;234;255;229m";
const ENDC: &str = "\x1b[0m";


pub fn true_clear() -> String {
  unsafe{ CURSOR_Y = 0 };
  unsafe{ CURSOR_X = 0 };
  let cleared: String = "\n".repeat(unsafe{ HEIGHT });
  let ups: String = UP.repeat(unsafe{ HEIGHT });
  return cleared + &ups + "\r";
}

pub fn clear() -> String {
  return true_clear() + "\r" + PREFIX;
}

pub fn clearline(len: usize) -> String {
  let right:String = RIGHT.repeat(len - unsafe{CURSOR_X});
  let out:String = RETURN.repeat(len);
  unsafe{ CURSOR_X = 0 };
  return right + &out;
}

pub fn echo(args: &str) -> String {
    unsafe {CURSOR_Y += 1};
    return NEWLINE.to_string() + args + NEWLINE + PREFIX;
}

pub fn ls(path_str: &str) -> String {
  let path = unsafe{PATH.path().join(path_str).display().to_string()};
  let resolved = resolve_path(&path);
  log(&resolved);
  let change:Option<&Dir>; 
  if resolved == ""{
    change = Some(&ROOT);
  } else {
    change = ROOT.get_dir(resolved);
  }  
  if !change.is_none(){
    let mut entries: Vec<String> = Vec::new();
    for entry in unsafe{ change.unwrap().entries() } {
      let name = entry.path().file_name().unwrap().to_string_lossy().to_string();
      if entry.as_dir().is_none() {
        entries.push(name);
      } else {
        entries.push(BLUE.to_string() + BOLD + &name + ENDC);
      }
    }
    return NEWLINE.to_string() + &entries.join(" ") + NEWLINE + PREFIX; 
  }
  return NEWLINE.to_string() + PREFIX; 
}

pub fn change_url(new_url: &str) -> Result<(), JsValue> {
    // Get a reference to the window's history object
    let window = window().expect("Should have a window in this context");
    let history = window.history().expect("Should have a history object in this context");

    // Push the new URL onto the history stack without reloading the page
    history.push_state_with_url(&JsValue::NULL, "", Some(new_url))
        .map_err(|err| err.into())
}

fn resolve_path(path: &str) -> String {
    let mut components: Vec<&str> = path.split('/').collect();
    let mut resolved_components: Vec<&str> = Vec::new();

    for component in components.iter() {
        if component == &".." {
            // If the component is '..', remove the last resolved component
            if !resolved_components.is_empty() {
                resolved_components.pop();
            }
        } else {
            // Otherwise, add the component to the resolved path
            resolved_components.push(component);
        }
    }

    resolved_components.join("/")
}

pub fn cd(path_str: &str) -> String {
  let path = unsafe{PATH.path().join(path_str).display().to_string()};
  let resolved = resolve_path(&path);
  log(&resolved);
  let change:Option<&Dir>; 
  if resolved == ""{
    change = Some(&ROOT);
  } else {
    change = ROOT.get_dir(resolved);
  }  
  if !change.is_none(){
    unsafe{ PATH = &change.unwrap() };
    let _ = change_url(&("/".to_string() + unsafe{PATH.path().to_str().unwrap()}));
  }
  return NEWLINE.to_string() + PREFIX; 
}

pub fn cat(path_str: &str) -> String {
  let path = unsafe{PATH.path().join(path_str).display().to_string()};
  log(&path);
  let resolved = resolve_path(&path);
  log(&resolved);
  let change:Option<&File>; 
  if resolved == ""{
    change = None;
  } else {
    change = ROOT.get_file(resolved);
  }  
  if !change.is_none(){
    log(change.unwrap().path().to_str().unwrap());
    return NEWLINE.to_string() + change.unwrap().contents_utf8().unwrap() + NEWLINE + PREFIX; 
  }
  return return format!("{}{}: No such file or directory{}{}", NEWLINE, path_str, NEWLINE.to_string(), PREFIX); 
}

pub fn pwd() -> String {
  return NEWLINE.to_string() + "/" + unsafe{ &PATH.path().display().to_string() } + NEWLINE + PREFIX; 
}


static mut LESS_LINE:usize = 0;

pub fn less_from(mut n: usize) -> String {
  let lines_len = unsafe{ LESS_LINES.len() };
  let bound: usize = if lines_len > unsafe{HEIGHT} {unsafe{lines_len - HEIGHT}} else {0};
  log(&format!("{} {}", n, bound));
  n = if n < bound {n} else {bound};
  log(&format!("{}", n));
  unsafe{LESS_LINE = n};
  let m:usize = if unsafe{n + HEIGHT - 1} < lines_len { unsafe{n + HEIGHT - 1} } else  { lines_len };
  let head: Vec<&str> = unsafe{LESS_LINES[n..m].to_vec()};
  let padding = unsafe{HEIGHT-head.len()};
  let suffix = if n == bound {WHITE_BACKGROUND.to_string() + BLACK + "(END)" + ENDC} else {":".to_string()};
  return NEWLINE.repeat(padding) +  &head.join("\r\n") + NEWLINE + &suffix; 
}

pub fn less_next() -> String {
  let lines_len = unsafe{ LESS_LINES.len() };
  let n = unsafe{LESS_LINE + 1};
  let bound: usize = if lines_len > unsafe{HEIGHT} {unsafe{lines_len - HEIGHT}} else {0};
  if n >= bound{
    return "".to_string();
  }
  unsafe{LESS_LINE = n};
  let m:usize = if unsafe{n + HEIGHT - 1} < lines_len { unsafe{n + HEIGHT - 1} } else  { lines_len };
  let line = unsafe{LESS_LINES[m]};
  let suffix = if n == bound {WHITE_BACKGROUND.to_string() + BLACK + "(END)" + ENDC} else {":".to_string()};
  return "\r".to_string() + line + NEWLINE + &suffix; 
}

pub fn less(path_str: &str) -> String {
  let path = unsafe{PATH.path().join(path_str).display().to_string()};
  log(&path);
  let resolved = resolve_path(&path);
  log(&resolved);
  let change:Option<&File>; 
  if resolved == ""{
    change = None;
  } else {
    change = ROOT.get_file(resolved);
  }  
  if !change.is_none(){
    log(change.unwrap().path().to_str().unwrap());
    unsafe{ LESS_LINES = change.unwrap().contents_utf8().unwrap().lines().collect() };
    unsafe{LESS = true};
    return less_from(0); 
  }

  return return format!("{}{}: No such file or directory{}{}", NEWLINE, path_str, NEWLINE.to_string(), PREFIX); 
}

pub fn help(args: &str) -> String {
    let help = "clear\t\tclear terminal \n\r\
            pwd\t\tprint current directory (or just check URL)\n\r\
            ls\t[PATH]\tlist files in directory\n\r\
            cd\tPATH\tchange directory\n\r\
            cat\tPATH\tstdout file\n\r\
            less\tPATH\tview file\n\r\
            echo\tMSG\techo message\n\r\
            help\t\tprint this message \
            ";
    return NEWLINE.to_string() + help + NEWLINE + PREFIX;
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
    "pwd" => return pwd(),
    "cd" => return cd(cmd_args.remainder().unwrap_or("")),
    "ls" => return ls(cmd_args.remainder().unwrap_or("")),
    "cat" => return cat(cmd_args.remainder().unwrap_or("")),
    "less" => return less(cmd_args.remainder().unwrap_or("")),
    "help" => return help(cmd_args.remainder().unwrap_or("")),
    "echo" => return echo(cmd_args.remainder().unwrap_or("")),
    _ => {
      return NEWLINE.to_string() + PREFIX
    }
  }
}

pub fn ansi(ansistr: &str, input_buffer: &mut MutexGuard<'_, Vec<char>>) -> String {
  let history = HISTORY.lock().unwrap();
  match ansistr {
    UP => {
      if unsafe{HISTORY_INDEX > 0} {
        unsafe{HISTORY_INDEX -= 1}
        let entry = history[unsafe{HISTORY_INDEX}]; 
        let out = clearline(input_buffer.len()) + entry;
        input_buffer.clear();
        input_buffer.extend(entry.chars());
        unsafe{CURSOR_X = entry.len()};
        return out;
      }
      return "".to_string();
    },
    DOWN => {
      if history.len() != 0 && unsafe{HISTORY_INDEX < history.len() - 1}{
        unsafe{HISTORY_INDEX += 1}
        let entry = history[unsafe{HISTORY_INDEX}]; 
        let out = clearline(input_buffer.len()) + entry;
        input_buffer.clear();
        input_buffer.extend(entry.chars());
        unsafe{CURSOR_X = entry.len()};
        return out;
      }
      let out = clearline(input_buffer.len());
      input_buffer.clear();
      unsafe{CURSOR_X = 0};
      return out;
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
    if unsafe{LESS} {
      return less_readchar(input);
    } else {
      return sh_readchar(input);
    }
}

pub fn less_readchar(input: char) -> String {
    let mut ansi_buffer = ANSI_BUFFER.lock().unwrap();
    if unsafe{ANSI} {
      ansi_buffer.push(input);
      let ansistr: String = ansi_buffer.iter().collect();
      match &ansistr as &str {
        UP => {
          unsafe{ ANSI = false };
          ansi_buffer.clear();
          return less_from(unsafe{if LESS_LINE > 0 {LESS_LINE - 1} else {0}});
        },
        DOWN => {
          unsafe{ ANSI = false };
          ansi_buffer.clear();
          return less_from(unsafe{LESS_LINE + 1});
        },
        PAGE_UP => {
          unsafe{ ANSI = false };
          ansi_buffer.clear();
          return less_from(unsafe{if LESS_LINE > HEIGHT {LESS_LINE - HEIGHT} else {0}});
        },
        PAGE_DOWN => {
          unsafe{ ANSI = false };
          ansi_buffer.clear();
          return less_from(unsafe{LESS_LINE + HEIGHT});
        },
        PAGE_START => {
          unsafe{ ANSI = false };
          ansi_buffer.clear();
          return less_from(0);
        },
        PAGE_END => {
          unsafe{ ANSI = false };
          ansi_buffer.clear();
          return less_from(usize::MAX);
        },
        RIGHT | LEFT => {
          unsafe{ ANSI = false };
          ansi_buffer.clear();
          return "".to_string();
        },
        _ => {},
      }
      return "".to_string()
    }
    match input {
      // ansi
      '\x1b' => {
        unsafe { ANSI = true };
        ansi_buffer.push(input);
        return "".to_string();
      },
      // quit
      'q' => {
        unsafe { LESS = false };
        return clear();
      },
      // top
      'g' => {
        return less_from(0);
      },
      // bottom
      'G' => {
        return less_from(usize::MAX);
      },
      _ => {
        return "".to_string();
      }
    }
}

pub fn sh_readchar(input: char) -> String {
    let mut ansi_buffer = ANSI_BUFFER.lock().unwrap();
    let mut input_buffer = INPUT_BUFFER.lock().unwrap();
    if unsafe{ANSI} {
      ansi_buffer.push(input);
      let ansistr: String = ansi_buffer.iter().collect();
      let out: String = ansi(&ansistr, &mut input_buffer);
      if out != "" || ansi_buffer.len() == 3 {
        unsafe { ANSI=false };
        ansi_buffer.clear(); 
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
        let len = input_buffer.len();
        input_buffer.clear();
        return clearline(len);
      },
      // clear
      '\x0c' => {
        input_buffer.clear();
        return clear();
      },
      // TAB
      '\x09' => {
        input_buffer.extend("  ".chars());
        unsafe{ CURSOR_X += 2 };
        return "  ".to_string();
      },
      // return key
      '\x7f' => {
          if input_buffer.is_empty() {
            return "".to_string();
          }
          let cursor_x = unsafe{ CURSOR_X - 1 };
          input_buffer.remove(cursor_x);
          let left = LEFT.repeat(input_buffer.len() - cursor_x);
          let inputstr: String = input_buffer.iter().collect();
          let out = clearline(input_buffer.len() + 1) + inputstr.as_str() + &left; 
          unsafe{ CURSOR_X = cursor_x };
          return out;
      },
      // ansi
      '\x1b' => {
        unsafe { ANSI = true };
        ansi_buffer.push(input);
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
