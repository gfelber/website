#![feature(str_split_remainder)]


mod utils;

use include_dir::{include_dir, Dir, File};
use wasm_bindgen::prelude::*;
use wasm_bindgen::JsValue;
use ansi_term::Colour;
use ansi_term::Style;
use web_sys::window;


#[wasm_bindgen]
extern "C" {
  #[wasm_bindgen(js_namespace = console)]
  fn log(s: &str);

  #[wasm_bindgen(js_namespace = console, js_name = log)]
  fn log_u32(a: u32);

  #[wasm_bindgen(js_namespace = console, js_name = log)]
  fn log_char(a: Option<char>);
}

const ROOT: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/root");
const PREFIX: &str = "$ ";

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


#[wasm_bindgen]
pub struct Term {
  path: &'static Dir<'static>,
  less_lines: Vec<&'static str>,
  history: Vec<&'static str>,
  input_buffer: Vec<char>,
  ansi_buffer: Vec<char>,
  history_index: usize,
  less_line: usize,
  cursor_x: usize,
  cursor_y: usize,
  height: usize,
  width: usize,
  less: bool,
  ansi: bool,
}

#[wasm_bindgen]
impl Term {
  #[wasm_bindgen(constructor)]
  pub fn new() -> Self {
    return Self {
      path: &ROOT,
      less_lines: vec![],
      history: vec![],
      input_buffer: vec![],
      ansi_buffer: vec![],
      history_index: 0,
      less_line: 0,
      cursor_x: 0,
      cursor_y: 0,
      height: 0,
      width: 0,
      less: false,
      ansi: false,
    };
  }

  pub fn init(&mut self, height: usize, width: usize, location: &str) -> String {
    utils::set_panic_hook();
    let mut location_str = location.to_string();
    location_str.remove(0);
    let path = ROOT.get_entry(location_str.clone());
    self.width = width - PREFIX.len(); // remove because of PREFIX
    self.height = height;
    if !path.is_none() {
      if path.unwrap().as_dir().is_none() {
        let resolved = Term::resolve_path(&(location_str + "/.."));
        log("works");
        log(&resolved);
        if !resolved.is_empty() {
          self.path = ROOT.get_dir(resolved).unwrap();
        }
        self.less = true;
        self.less_lines = path.unwrap().as_file().unwrap().contents_utf8().unwrap().lines().collect();
        return self.less_from(0);
      } else {
        self.path = path.unwrap().as_dir().unwrap();
      }
    }
    return PREFIX.to_string();
  }


  pub fn readline(&mut self, input: &str) -> String {
    let mut vec = Vec::<String>::new();
    for c in input.chars() {
      vec.push(self.readchar(c));
    }
    return vec.join("");
  }


  pub fn true_clear(&mut self) -> String {
    self.cursor_y = 0;
    self.cursor_x = 0;
    let cleared: String = "\n".repeat(self.height);
    let ups: String = UP.repeat(self.height);
    return cleared + &ups + "\r";
  }

  pub fn clear(&mut self) -> String {
    return self.true_clear() + "\r" + PREFIX;
  }

  pub fn clearline(&mut self, len: usize) -> String {
    let right: String = RIGHT.repeat(len - self.cursor_x);
    let out: String = RETURN.repeat(len);
    self.cursor_x = 0;
    return right + &out;
  }

  pub fn echo(&mut self, args: &str) -> String {
    self.cursor_y += 1;
    return NEWLINE.to_string() + args + NEWLINE + PREFIX;
  }

  pub fn ls(&mut self, path_str: &str) -> String {
    let path = self.path.path().join(path_str).display().to_string();
    let resolved = Term::resolve_path(&path);
    log(&resolved);
    let change: Option<&Dir>;
    if resolved == "" {
      change = Some(&ROOT);
    } else {
      change = ROOT.get_dir(resolved);
    }
    if !change.is_none() {
      let mut entries: Vec<String> = Vec::new();
      for entry in change.unwrap().entries() {
        let name = entry.path().file_name().unwrap().to_string_lossy().to_string();
        if entry.as_dir().is_none() {
          entries.push(name);
        } else {
          entries.push(Colour::Blue.bold().paint(&name).to_string());
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
    let components: Vec<&str> = path.split('/').collect();
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

  pub fn cd(&mut self, path_str: &str) -> String {
    let mut path = "".to_string();
    if !(path_str.is_empty() || path_str == "/") {
      if path_str.starts_with('/') {
        path = path_str[1..].to_string();
      } else {
        path = self.path.path().join(path_str).display().to_string();
      }
    }
    log(&path);
    let resolved = Term::resolve_path(&path);
    log(&resolved);
    let change: Option<&Dir>;
    if resolved.is_empty() {
      change = Some(&ROOT);
    } else {
      change = ROOT.get_dir(resolved);
    }
    if !change.is_none() {
      self.path = &change.unwrap();
      let _ = Term::change_url(&("/".to_string() + self.path.path().to_str().unwrap()));
    }
    return NEWLINE.to_string() + PREFIX;
  }

  pub fn cat(&mut self, path_str: &str) -> String {
    let path = self.path.path().join(path_str).display().to_string();
    log(&path);
    let resolved = Term::resolve_path(&path);
    log(&resolved);
    let change: Option<&File>;
    if resolved == "" {
      change = None;
    } else {
      change = ROOT.get_file(resolved);
    }
    if !change.is_none() {
      log(change.unwrap().path().to_str().unwrap());
      return NEWLINE.to_string() + change.unwrap().contents_utf8().unwrap() + NEWLINE + PREFIX;
    }
    return format!("{}{}: No such file or directory{}{}", NEWLINE, path_str, NEWLINE.to_string(), PREFIX);
  }

  pub fn pwd(&mut self, _args: &str) -> String {
    return NEWLINE.to_string() + "/" + &self.path.path().display().to_string() + NEWLINE + PREFIX;
  }

  pub fn less_from(&mut self, mut n: usize) -> String {
    let lines_len = self.less_lines.len();
    let bound: usize = if lines_len > self.height { lines_len - self.height } else { 0 };
    log(&format!("{} {}", n, bound));
    n = if n < bound { n } else { bound };
    log(&format!("{}", n));
    self.less_line = n;
    let m: usize = if n + self.height - 1 < lines_len { n + self.height - 1 } else { lines_len };
    let head: Vec<&str> = self.less_lines[n..m].to_vec();
    let padding = self.height - head.len();
    let suffix = if n == bound {
      Style::new().on(Colour::RGB(234, 255, 229))
        .fg(Colour::Black)
        .paint("(END)").to_string()
    } else {
      ":".to_string()
    };
    return NEWLINE.repeat(padding) + &head.join("\r\n") + NEWLINE + &suffix;
  }

  pub fn less(&mut self, path_str: &str) -> String {
    let path = self.path.path().join(path_str).display().to_string();
    log(&path);
    let resolved = Term::resolve_path(&path);
    log(&resolved);
    let change: Option<&File>;
    if resolved == "" {
      change = None;
    } else {
      change = ROOT.get_file(resolved);
    }
    if !change.is_none() {
      let _ = Term::change_url(&("/".to_string() + change.unwrap().path().to_str().unwrap()));
      log(change.unwrap().path().to_str().unwrap());
      self.less_lines = change.unwrap().contents_utf8().unwrap().lines().collect();
      self.less = true;
      return self.less_from(0);
    }

    return format!("{}{}: No such file or directory{}{}", NEWLINE, path_str, NEWLINE.to_string(), PREFIX);
  }

  pub fn help(&mut self, _args: &str) -> String {
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


  pub fn command(&mut self, cmdline: &str) -> String {
    self.history.push(Box::leak(cmdline.to_owned().into_boxed_str()));
    self.history_index = self.history.len();
    let mut cmd_args = cmdline.split(" ");
    let cmdline = cmd_args.next().unwrap();
    self.cursor_y += 1;
    return match cmdline {
      "clear" => self.clear(),
      "pwd" => self.pwd(cmd_args.remainder().unwrap_or("")),
      "cd" => self.cd(cmd_args.remainder().unwrap_or("")),
      "ls" => self.ls(cmd_args.remainder().unwrap_or("")),
      "cat" => self.cat(cmd_args.remainder().unwrap_or("")),
      "less" => self.less(cmd_args.remainder().unwrap_or("")),
      "help" => self.help(cmd_args.remainder().unwrap_or("")),
      "echo" => self.echo(cmd_args.remainder().unwrap_or("")),
      _ => {
        NEWLINE.to_string() + PREFIX
      }
    };
  }

  pub fn ansi(&mut self, ansistr: &str) -> String {
    match ansistr {
      UP => {
        if self.history_index > 0 {
          self.history_index -= 1;
          let entry = self.history[self.history_index];
          let out = self.clearline(self.input_buffer.len()) + entry;
          self.input_buffer.clear();
          self.input_buffer.extend(entry.chars());
          self.cursor_x = entry.len();
          return out;
        }
        return "".to_string();
      }
      DOWN => {
        if self.history.len() != 0 && self.history_index < self.history.len() - 1 {
          self.history_index += 1;
          let entry = self.history[self.history_index];
          let out = self.clearline(self.input_buffer.len()) + entry;
          self.input_buffer.clear();
          self.input_buffer.extend(entry.chars());
          self.cursor_x = entry.len();
          return out;
        }
        let out = self.clearline(self.input_buffer.len());
        self.input_buffer.clear();
        self.cursor_x = 0;
        return out;
      }
      RIGHT => {
        if self.cursor_x < self.input_buffer.len() {
          self.cursor_x += 1;
          return RIGHT.to_string();
        }
      }
      LEFT => {
        if self.cursor_x >= PREFIX.len() {
          self.cursor_x -= 1;
          return LEFT.to_string();
        }
      }
      PAGE_START => {
        let repeat = self.cursor_x;
        self.cursor_x = 0;
        return LEFT.repeat(repeat);
      }
      PAGE_END => {
        let repeat = self.input_buffer.len() - self.cursor_x;
        self.cursor_x = self.input_buffer.len();
        return RIGHT.repeat(repeat);
      }
      _ => {}
    }
    return "".to_string();
  }

  pub fn readchar(&mut self, input: char) -> String {
    log(&format!("{:02x}", input as u32));
    return if self.less {
      self.less_readchar(input)
    } else {
      self.sh_readchar(input)
    };
  }

  pub fn less_readchar(&mut self, input: char) -> String {
    if self.ansi {
      self.ansi_buffer.push(input);
      let ansistr: String = self.ansi_buffer.iter().collect();
      match &ansistr as &str {
        UP => {
          self.ansi = false;
          self.ansi_buffer.clear();
          return self.less_from(if self.less_line > 0 { self.less_line - 1 } else { 0 });
        }
        DOWN => {
          self.ansi = false;
          self.ansi_buffer.clear();
          return self.less_from(self.less_line + 1);
        }
        PAGE_UP => {
          self.ansi = false;
          self.ansi_buffer.clear();
          return self.less_from(if self.less_line > self.height { self.less_line - self.height } else { 0 });
        }
        PAGE_DOWN => {
          self.ansi = false;
          self.ansi_buffer.clear();
          return self.less_from(self.less_line + self.height);
        }
        PAGE_START => {
          self.ansi = false;
          self.ansi_buffer.clear();
          return self.less_from(0);
        }
        PAGE_END => {
          self.ansi = false;
          self.ansi_buffer.clear();
          return self.less_from(usize::MAX);
        }
        RIGHT | LEFT => {
          self.ansi = false;
          self.ansi_buffer.clear();
          return "".to_string();
        }
        _ => {}
      }
      return "".to_string();
    }
    return match input {
      // ansi
      '\x1b' => {
        self.ansi = true;
        self.ansi_buffer.push(input);
        "".to_string()
      }
      // quit
      'q' => {
        let _ = Term::change_url(&("/".to_string() + self.path.path().to_str().unwrap()));
        self.less = false;
        self.clear()
      }
      // top
      'g' => {
        self.less_from(0)
      }
      // bottom
      'G' => {
        self.less_from(usize::MAX)
      }
      _ => {
        "".to_string()
      }
    };
  }

  pub fn sh_readchar(&mut self, input: char) -> String {
    if self.ansi {
      self.ansi_buffer.push(input);
      let ansistr: String = self.ansi_buffer.iter().collect();
      let out: String = self.ansi(&ansistr);
      if out != "" || self.ansi_buffer.len() == 3 {
        self.ansi = false;
        self.ansi_buffer.clear();
      }
      return out;
    }
    return match input {
      '\r' | '\n' => {
        let cmd: String = self.input_buffer.iter().collect();
        log(&cmd);
        self.input_buffer.clear();
        self.cursor_x = 0;
        self.command(&cmd)
      }
      // clear line
      '\x15' => {
        let len = self.input_buffer.len();
        self.input_buffer.clear();
        self.clearline(len)
      }
      // clear
      '\x0c' => {
        self.input_buffer.clear();
        self.clear()
      }
      // TAB
      '\x09' => {
        self.input_buffer.extend("  ".chars());
        self.cursor_x += 2;
        "  ".to_string()
      }
      // return key
      '\x7f' => {
        if self.input_buffer.is_empty() {
          return "".to_string();
        }
        let cursor_x = self.cursor_x - 1;
        self.input_buffer.remove(cursor_x);
        let left = LEFT.repeat(self.input_buffer.len() - cursor_x);
        let inputstr: String = self.input_buffer.iter().collect();
        let out = self.clearline(self.input_buffer.len() + 1) + inputstr.as_str() + &left;
        self.cursor_x = cursor_x;
        out
      }
      // ansi
      '\x1b' => {
        self.ansi = true;
        self.ansi_buffer.push(input);
        "".to_string()
      }
      _ => {
        if self.cursor_x < self.input_buffer.len() {
          self.input_buffer[self.cursor_x] = input;
        } else if self.cursor_x <= self.width {
          self.cursor_x += 1;
          self.input_buffer.push(input);
        } else {
          log("reached EOL");
          return "".to_string();
        }
        input.to_string()
      }
    };
  }
}

