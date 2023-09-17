use ansi_term::Colour;
use include_dir::{Dir, File};
use crate::app::App;
use crate::{consts, utils};
use crate::less::Less;
use crate::termstate::TermState;

const PREFIX: &str = "$ ";

pub struct Shell{
  history: Vec<&'static str>,
  input_buffer: Vec<char>,
  ansi_buffer: Vec<char>,
  history_index: usize,
  ansi: bool,
}

impl Shell{

  pub fn new() -> Self{
    Self{
      history: vec![],
      input_buffer: vec![],
      ansi_buffer: vec![],
      history_index: 0,
      ansi: false,
    }
  }

  pub fn clear(state:&mut TermState) -> String {
    let out = state.clear() + "\r" + PREFIX;
    state.cursor_x = PREFIX.len();
    return out;
  }

  fn clearline(&mut self, state:&mut TermState) -> String {
    let right: String = consts::RIGHT.repeat(self.input_buffer.len() - (state.cursor_x - PREFIX.len()));
    let out: String = consts::RETURN.repeat(self.input_buffer.len());
    state.cursor_x = PREFIX.len();
    return right + &out;
  }

  fn echo(&mut self, state: &mut TermState, args: &str) -> String {
    state.cursor_y += 2;
    return consts::NEWLINE.to_string() + args + consts::NEWLINE + PREFIX;
  }

  fn ls(&mut self, state: &mut TermState, path_str: &str) -> String {
    let path = state.path.path().join(path_str).display().to_string();
    let resolved = utils::resolve_path(&path);
    utils::log(&resolved);
    let change: Option<&Dir>;
    if resolved == "" {
      change = Some(&consts::ROOT);
    } else {
      change = consts::ROOT.get_dir(resolved);
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
      state.cursor_y += 2;
      state.cursor_x = PREFIX.len();
      return consts::NEWLINE.to_string() + &entries.join(" ") + consts::NEWLINE + PREFIX;
    }
    state.cursor_y += 1;
    state.cursor_x = PREFIX.len();
    return consts::NEWLINE.to_string() + PREFIX;
  }

  fn cd(&mut self, state: &mut TermState, path_str: &str) -> String {
    let mut path = "".to_string();
    if !(path_str.is_empty() || path_str == "/") {
      if path_str.starts_with('/') {
        path = path_str[1..].to_string();
      } else {
        path = state.path.path().join(path_str).display().to_string();
      }
    }
    utils::log(&path);
    let resolved = utils::resolve_path(&path);
    utils::log(&resolved);
    let change: Option<&Dir>;
    if resolved.is_empty() {
      change = Some(&consts::ROOT);
    } else {
      change = consts::ROOT.get_dir(resolved);
    }
    if change.is_some() {
      state.path = &change.unwrap();
      let _ = utils::change_url(&("/".to_string() + state.path.path().to_str().unwrap()));
      state.cursor_y += 1;
      state.cursor_x = PREFIX.len();
      return consts::NEWLINE.to_string() + PREFIX;
    }
    state.cursor_y += 2;
    state.cursor_x = PREFIX.len();
    return format!("{}{}: No such file or directory{}{}", consts::NEWLINE, path_str, consts::NEWLINE.to_string(), PREFIX);
  }

  fn cat(&mut self, state: &mut TermState, path_str: &str) -> String {
    let path = state.path.path().join(path_str).display().to_string();
    utils::log(&path);
    let resolved = utils::resolve_path(&path);
    utils::log(&resolved);
    let change: Option<&File>;
    if resolved == "" {
      change = None;
    } else {
      change = consts::ROOT.get_file(resolved);
    }
    if !change.is_none() {
      utils::log(change.unwrap().path().to_str().unwrap());
      state.cursor_y += change.unwrap().contents_utf8().unwrap().lines().count();
      state.cursor_x = PREFIX.len() + 2;
      return consts::NEWLINE.to_string() + change.unwrap().contents_utf8().unwrap() + consts::NEWLINE + PREFIX;
    }
    state.cursor_y += 2;
    state.cursor_x = PREFIX.len();
    return format!("{}{}: No such file or directory{}{}", consts::NEWLINE, path_str, consts::NEWLINE.to_string(), PREFIX);
  }

  fn pwd(&mut self, state: &mut TermState, _args: &str) -> String {
    return consts::NEWLINE.to_string() + "/" + &state.path.path().display().to_string() + consts::NEWLINE + PREFIX;
  }

  fn help(&mut self, _state: &mut TermState, _args: &str) -> String {
    let help = "\
            clear\t\tclear terminal\n\r\
            pwd\t\tprint current directory (or just check URL)\n\r\
            ls\t[PATH]\tlist files in directory\n\r\
            cd\tPATH\tchange directory\n\r\
            cat\tPATH\tstdout file\n\r\
            less\tPATH\tview file\n\r\
            echo\tMSG\techo message\n\r\
            help\t\tprint this message\
            ";
    return consts::NEWLINE.to_string() + help + consts::NEWLINE + PREFIX;
  }

  fn less(&mut self, state: &mut TermState, args: &str) -> (Option<Box<dyn App>>, String){
    let mut less = Less::new();
    return match less.less(state, args) {
      Ok(result) => (Some(Box::new(less)), result),
      Err(error) => {
        state.cursor_x = PREFIX.len();
        state.cursor_y += 2;
        (None, error + PREFIX)
      }

    }
  }


  fn command(&mut self, state: &mut TermState, cmdline: &str) -> (Option<Box<dyn App>>, String) {
    self.history.push(Box::leak(cmdline.to_owned().into_boxed_str()));
    self.history_index = self.history.len();
    let mut cmd_args = cmdline.split(" ");
    let cmdline = cmd_args.next().unwrap();
    return match cmdline {
      "clear" => (None, Shell::clear(state)),
      "pwd" => (None, self.pwd(state, cmd_args.remainder().unwrap_or(""))),
      "cd" => (None, self.cd(state, cmd_args.remainder().unwrap_or(""))),
      "ls" => (None, self.ls(state, cmd_args.remainder().unwrap_or(""))),
      "cat" => (None, self.cat(state, cmd_args.remainder().unwrap_or(""))),
      "less" => self.less(state, cmd_args.remainder().unwrap_or("")),
      "help" => (None, self.help(state, cmd_args.remainder().unwrap_or(""))),
      "echo" => (None, self.echo(state, cmd_args.remainder().unwrap_or(""))),
      _ => {
        state.cursor_y += 1;
        (None, consts::NEWLINE.to_string() + PREFIX)
      }
    };
  }


  fn ansi(&mut self, state: &mut TermState, ansistr: &str) -> String {
    match ansistr {
      consts::UP => {
        if self.history_index > 0 {
          self.history_index -= 1;
          let entry = self.history[self.history_index];
          let out = self.clearline(state) + entry;
          self.input_buffer.clear();
          self.input_buffer.extend(entry.chars());
          state.cursor_x = entry.len() + PREFIX.len();
          return out;
        }
        return "".to_string();
      }
      consts::DOWN => {
        if self.history.len() != 0 && self.history_index < self.history.len() - 1 {
          self.history_index += 1;
          let entry = self.history[self.history_index];
          let out = self.clearline(state) + entry;
          self.input_buffer.clear();
          self.input_buffer.extend(entry.chars());
          state.cursor_x = entry.len();
          return out;
        }
        let out = self.clearline(state);
        self.input_buffer.clear();
        state.cursor_x = PREFIX.len();
        return out;
      }
      consts::RIGHT => {
        if state.cursor_x < self.input_buffer.len() + PREFIX.len() {
          state.cursor_x += 1;
          return consts::RIGHT.to_string();
        }
      }
      consts::LEFT => {
        if state.cursor_x > PREFIX.len() {
          state.cursor_x -= 1;
          return consts::LEFT.to_string();
        }
      }
      consts::PAGE_START => {
        let repeat = state.cursor_x - PREFIX.len();
        state.cursor_x = PREFIX.len();
        return consts::LEFT.repeat(repeat);
      }
      consts::PAGE_END => {
        let repeat = self.input_buffer.len() - state.cursor_x - PREFIX.len();
        state.cursor_x = self.input_buffer.len() + PREFIX.len();
        return consts::RIGHT.repeat(repeat);
      }
      _ => {}
    }
    return "".to_string();
  }
}

impl App for Shell{
  fn readchar(&mut self, state: &mut TermState, mut input: char) -> (Option<Box<dyn App>>, String) {
    if self.ansi {
      self.ansi_buffer.push(input);
      let ansistr: String = self.ansi_buffer.iter().collect();
      let out: String = self.ansi(state, &ansistr);
      if out != "" || self.ansi_buffer.len() == 3 {
        self.ansi = false;
        self.ansi_buffer.clear();
      }
      return (None, out);
    }
    return match input {
      '\r' | '\n' => {
        let cmd: String = self.input_buffer.iter().collect();
        utils::log(&cmd);
        self.input_buffer.clear();
        state.cursor_x = PREFIX.len();
        self.command(state, &cmd)
      }
      // clear line
      '\x15' => {
        let out = self.clearline(state);
        self.input_buffer.clear();
        state.cursor_x = PREFIX.len();
        (None, out + PREFIX)
      }
      // clear
      '\x0c' => {
        self.input_buffer.clear();
        (None, Shell::clear(state))
      }
      // return key
      '\x7f' => {
        if self.input_buffer.is_empty() {
          return (None, "".to_string())
        }
        let cursor_x = state.cursor_x - (PREFIX.len() + 1);
        utils::log(&format!("{}/{}", cursor_x, self.input_buffer.len()));
        let left = consts::LEFT.repeat(self.input_buffer.len() - cursor_x);
        let mut out = self.clearline(state);
        self.input_buffer.remove(cursor_x);
        let inputstr: String = self.input_buffer.iter().collect();
        out = out + inputstr.as_str() + " " + &left;
        state.cursor_x = cursor_x + PREFIX.len();
        (None, out)
      }
      // ansi
      '\x1b' => {
        self.ansi = true;
        self.ansi_buffer.push(input);
        (None, "".to_string())
      }
      _ => {
        // TAB change to whitespace
        if input == '\x09'{
          input = ' ';
        }
        if state.cursor_x < self.input_buffer.len() + PREFIX.len() {
          self.input_buffer[state.cursor_x - PREFIX.len()] = input;
        } else if state.cursor_x <= state.width {
          state.cursor_x += 1;
          self.input_buffer.push(input);
        } else {
          utils::log("reached EOL");
          return (None, "".to_string());
        }
        (None, input.to_string())
      }
    };
  }
}