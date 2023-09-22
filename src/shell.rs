use std::ffi::OsStr;
use ansi_term::Colour;
use include_dir::{Dir, DirEntry, File};
use crate::app::App;
use crate::{consts, utils};
use crate::less::Less;
use crate::termstate::TermState;
use log::info;
use clap::Parser;

const PREFIX: &str = "$ ";
const DIR_PREFIX: &str = "dr-xr-xr-x\t2 root\troot\t4069\tJan 1 1970 ";
const FILE_PREFIX: &str = "-r--r--r--\t1 root\troot\t1337\tJan 1 1970 ";

macro_rules! parse_args {
  ($state:expr, $e:expr) => {
    match $e{
      Ok(args) => {args}
      Err(error) => {
        let error_str = error.to_string();
        let lines: Vec<&str> = error_str.lines().collect();
        $state.cursor_x = PREFIX.len();
        $state.cursor_y += lines.len() + 2;
        return consts::NEWLINE.to_string() + &lines.join(consts::NEWLINE) + consts::NEWLINE + PREFIX;
      }

    }
  }
}

#[derive(Parser)]
#[command(about = "List directory contents")]
struct Ls {
  #[arg(hide_short_help = true, hide_long_help = true)]
  file: Option<String>,
  #[arg(short = 'R', long, action, help = "recursive")]
  recursive: bool,
  #[arg(short, long, action, help = "list directory names, not contents")]
  directory: bool,
  #[arg(short, action, help = "long format")]
  list: bool,
}

pub struct Shell {
  history: Vec<&'static str>,
  input_buffer: Vec<char>,
  ansi_buffer: Vec<char>,
  history_index: usize,
  ansi: bool,
}

impl App for Shell {
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
        info!("{}", cmd);
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
          return (None, "".to_string());
        }
        let cursor_x = state.cursor_x - (PREFIX.len() + 1);
        info!("{}/{}", cursor_x, self.input_buffer.len());
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
        if input == '\x09' {
          input = ' ';
        }
        if state.cursor_x < self.input_buffer.len() + PREFIX.len() {
          self.input_buffer[state.cursor_x - PREFIX.len()] = input;
        } else if state.cursor_x <= state.width {
          state.cursor_x += 1;
          self.input_buffer.push(input);
        } else {
          info!("reached EOL");
          return (None, "".to_string());
        }
        (None, input.to_string())
      }
    };
  }
}

impl Shell {
  pub fn new() -> Self {
    Self {
      history: vec![],
      input_buffer: vec![],
      ansi_buffer: vec![],
      history_index: 0,
      ansi: false,
    }
  }

  pub fn clear(state: &mut TermState) -> String {
    let out = state.clear() + "\r" + PREFIX;
    state.cursor_x = PREFIX.len();
    return out;
  }

  fn clearline(&mut self, state: &mut TermState) -> String {
    let right: String = consts::RIGHT.repeat(self.input_buffer.len() - (state.cursor_x - PREFIX.len()));
    let out: String = consts::RETURN.repeat(self.input_buffer.len());
    state.cursor_x = PREFIX.len();
    return right + &out;
  }

  fn echo(&mut self, state: &mut TermState, args: &str) -> String {
    state.cursor_y += 2;
    return consts::NEWLINE.to_string() + args + consts::NEWLINE + PREFIX;
  }

  fn ls(&mut self, state: &mut TermState, cmdline: &str) -> String {
    let lsargs: Ls = parse_args!(state, Ls::try_parse_from(cmdline.split(" ")));
    let path_str = lsargs.file.unwrap_or(".".to_string());
    let path = state.path.path().join(path_str.clone()).display().to_string();
    let resolved = utils::resolve_path(&path);
    info!("{}", resolved);
    let change: Option<&DirEntry> = if !resolved.is_empty() {
      consts::ROOT.get_entry(resolved.clone())
    } else {
      None
    };
    if resolved.is_empty() || change.is_some() {
      if resolved.is_empty() || change.unwrap().as_dir().is_some() {
        let dir: &Dir = if resolved.is_empty() {
          &consts::ROOT
        } else {
          &change.unwrap().as_dir().unwrap()
        };
        if lsargs.directory {
          state.cursor_x = PREFIX.len();
          state.cursor_y += 2;
          let formatted_filename = Colour::Blue.bold().paint(path_str).to_string();
          if lsargs.list {
            return consts::NEWLINE.to_string() + DIR_PREFIX + &formatted_filename + consts::NEWLINE + PREFIX;
          } else {
            return consts::NEWLINE.to_string() + &formatted_filename + consts::NEWLINE + PREFIX;
          }
        }
        let prefix = if lsargs.recursive {
          path_str.clone() + ":" + consts::NEWLINE
        } else {
          "".to_string()
        };
        let mut totalsize = 0;
        let mut entries: Vec<String> = Vec::new();
        let mut recursive_dirs: Vec<String> = Vec::new();
        for entry in dir.entries() {
          let name = entry.path().file_name().unwrap().to_string_lossy().to_string();
          if entry.as_file().is_some() {
            if lsargs.list {
              state.cursor_y += 1;
              totalsize += 1337;
              entries.push(FILE_PREFIX.to_string() + &name + consts::NEWLINE);
            } else {
              entries.push(name);
            }
          } else {
            if lsargs.recursive {
              recursive_dirs.push(name.clone());
            }
            if lsargs.list {
              state.cursor_y += 1;
              totalsize += 4096;
              entries.push(DIR_PREFIX.to_string() + &Colour::Blue.bold().paint(&name).to_string() + consts::NEWLINE);
            } else {
              entries.push(Colour::Blue.bold().paint(&name).to_string());
            }
          }
        }
        if !lsargs.list{
          entries.push(consts::NEWLINE.to_string());
        }
        state.cursor_x = PREFIX.len();
        state.cursor_y += 2;
        if lsargs.recursive {
          for entry in recursive_dirs {
            let options = if lsargs.list {
              "-lR"
            } else {
              "-R"
            };
            let file = &format!("{}/{}", path_str, entry);
            let mut out = self.ls(state, &format!("ls {} {}", options, file));
            out.truncate(out.len() - PREFIX.len());
            entries.push(out);
          }
        }
        if lsargs.list {
          return format!("{}{}total {}{}{}{}", consts::NEWLINE, prefix, totalsize, consts::NEWLINE, &entries.join(""), PREFIX);
        } else {
          return consts::NEWLINE.to_string() + &prefix + &entries.join("\t") + PREFIX;
        }
      } else {
        state.cursor_x = PREFIX.len();
        state.cursor_y += 2;
        let filename = &change.unwrap().path().file_name().unwrap_or(OsStr::new(".")).to_string_lossy().to_string();
        if lsargs.list {
          return consts::NEWLINE.to_string() + FILE_PREFIX + &filename + consts::NEWLINE + PREFIX;
        } else {
          return consts::NEWLINE.to_string() + &filename + consts::NEWLINE + PREFIX;
        }
      }
    }
    state.cursor_y += 2;
    state.cursor_x = PREFIX.len();
    return format!("{}{}: No such file or directory{}{}", consts::NEWLINE, path_str, consts::NEWLINE.to_string(), PREFIX);
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
    info!("{}", path);
    let resolved = utils::resolve_path(&path);
    info!("{}", resolved);
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
    info!("{}", path);
    let resolved = utils::resolve_path(&path);
    info!("{}", resolved);
    let change: Option<&File>;
    if resolved == "" {
      change = None;
    } else {
      change = consts::ROOT.get_file(resolved);
    }
    if !change.is_none() {
      info!("{}", change.unwrap().path().to_str().unwrap());
      let lines: Vec<&str> = change.unwrap().contents_utf8().unwrap().lines().collect();
      state.cursor_y += lines.len() + 2;
      state.cursor_x = PREFIX.len() + 2;
      return consts::NEWLINE.to_string() + &lines.join(consts::NEWLINE) + consts::NEWLINE + PREFIX;
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

  fn less(&mut self, state: &mut TermState, args: &str) -> (Option<Box<dyn App>>, String) {
    let mut less = Less::new();
    return match less.less(state, args) {
      Ok(result) => (Some(Box::new(less)), result),
      Err(error) => {
        state.cursor_x = PREFIX.len();
        state.cursor_y += 2;
        (None, error + PREFIX)
      }
    };
  }


  fn command(&mut self, state: &mut TermState, cmdline: &str) -> (Option<Box<dyn App>>, String) {
    self.history.push(Box::leak(cmdline.to_owned().into_boxed_str()));
    self.history_index = self.history.len();
    let mut cmd_args = cmdline.split(" ");
    let cmd = cmd_args.next().unwrap();
    return match cmd {
      "clear" => (None, Shell::clear(state)),
      "pwd" => (None, self.pwd(state, cmd_args.remainder().unwrap_or(""))),
      "cd" => (None, self.cd(state, cmd_args.remainder().unwrap_or(""))),
      "ls" => (None, self.ls(state, cmdline)),
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