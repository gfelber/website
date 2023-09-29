use std::sync::Mutex;

use ansi_term::Colour;
use clap::{ArgAction, Parser};
use lazy_static::lazy_static;
use log::{info, warn};

use crate::{consts, filesystem, utils};
use crate::app::App;
use crate::less::Less;
use crate::termstate::TermState;

const PREFIX: &str = "$ ";
const DIR_PREFIX: &str = "dr-xr-xr-x\t2 root\troot";
const FILE_PREFIX: &str = "-r--r--r--\t1 root\troot";

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
#[command(about = "list directory contents", disable_help_flag = true)]
struct LsArgs {
  #[arg(hide_short_help = true, hide_long_help = true)]
  file: Option<String>,
  #[arg(short = 'R', long, action, help = "recursive")]
  recursive: bool,
  #[arg(short, long, action, help = "list directory names, not contents")]
  directory: bool,
  #[arg(short, action, help = "Human readable sizes (1K 243M 2G)")]
  human: bool,
  #[arg(long, global = true, action = ArgAction::HelpShort, hide_short_help = true, hide_long_help = true)]
  help: Option<bool>,
  #[arg(short, action, help = "long format")]
  list: bool,
}

#[derive(Parser)]
#[command(about = "change directory")]
struct CdArgs {
  #[arg(help = "directory to change into")]
  dir: Option<String>
}

#[derive(Parser)]
#[command(about = "print file to stdout")]
struct CatArgs {
  #[arg(help = "file to print")]
  file: String
}

#[derive(Parser)]
#[command(about = "view file inside screen")]
struct LessArgs {
  #[arg(help = "file to view")]
  file: String
}

lazy_static! {
    static ref CMD_HISTORY: Mutex<Vec<&'static str>> = Mutex::new(vec![]);
}

pub struct Shell {
  input_buffer: Vec<char>,
  ansi_buffer: Vec<char>,
  history_index: usize,
  ansi: bool,
  insert: bool,
}

impl App for Shell {
  fn readchar(&mut self, state: &mut TermState, mut input: char) -> (Option<Box<dyn App>>, String) {
    if self.ansi {
      self.ansi_buffer.push(input);
      let ansistr: String = self.ansi_buffer.iter().collect();
      let mut hex = "".to_string();
      for byt in ansistr.as_bytes(){
        hex += &format!("{:02X}", byt);
      }
      info!("{}", hex);
      let out: String = self.ansi(state, &ansistr);
      if out != "" || self.ansi_buffer.len() == 4 {
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
        let mut out = self.clearline(state) + PREFIX;
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
      // only printable characters
      c if c >= ' ' => {
        // TAB change to whitespace
        if input == '\x09' {
          input = ' ';
        }
        if state.cursor_x < self.input_buffer.len() + PREFIX.len() {
          if self.insert{
            self.input_buffer[state.cursor_x - PREFIX.len()] = input;
          } else {
            self.input_buffer.insert(state.cursor_x - PREFIX.len(), input);
            let new_x = state.cursor_x + 1;
            let input_str: String = self.input_buffer.iter().collect();
            let left = consts::LEFT.repeat(self.input_buffer.len() - (new_x - PREFIX.len()));
            let out = self.clearline(state) + PREFIX + &input_str + &left;
            state.cursor_x = new_x;
            return (None, out);
          }
        } else if state.cursor_x < state.width - 1 {
          state.cursor_x += 1;
          self.input_buffer.push(input);
        } else {
          info!("reached EOL");
          return (None, "".to_string());
        }
        (None, input.to_string())
      }
      _ => {
        warn!("character not supported: {:2x}", input as u64);
        (None, "".to_string())
      }
    };
  }
}

impl Shell {
  pub fn new() -> Self {
    let history = CMD_HISTORY.lock().unwrap();
    Self {
      input_buffer: vec![],
      ansi_buffer: vec![],
      history_index: history.len(),
      ansi: false,
      insert: false,
    }
  }

  pub fn clear(state: &mut TermState) -> String {
    let out = state.clear() + "\r" + PREFIX;
    state.cursor_x = PREFIX.len();
    return out;
  }

  fn clearline(&mut self, state: &mut TermState) -> String {
    let right: String = consts::RIGHT.repeat(self.input_buffer.len() - (state.cursor_x - PREFIX.len()));
    let out: String = consts::RETURN.repeat(self.input_buffer.len() + PREFIX.len());
    state.cursor_x = 0;
    return right + &out;
  }

  fn echo(&mut self, state: &mut TermState, args: &str) -> String {
    state.cursor_y += 2;
    return consts::NEWLINE.to_string() + args + consts::NEWLINE + PREFIX;
  }

  fn whoami(&mut self, state: &mut TermState, _args: &str) -> String {
    state.cursor_y += 2;
    return consts::NEWLINE.to_string() + "user" + consts::NEWLINE + PREFIX;
  }

  fn ls(&mut self, state: &mut TermState, cmdline: &str) -> String {
    let lsargs: LsArgs = parse_args!(state, LsArgs::try_parse_from(cmdline.split(" ")));
    let path_str = lsargs.file.unwrap_or(".".to_string());
    let path = state.path.join(path_str.clone());
    let resolved = utils::resolve_path(&path);
    info!("{}", resolved);
    let change = filesystem::ROOT.get_file(&resolved);
    if resolved.is_empty() || change.is_ok() {
      if !lsargs.directory && (resolved.is_empty() || change.clone().unwrap().is_dir) {
        let dir = if resolved.is_empty() {
          &filesystem::ROOT
        } else {
          change.unwrap()
        };
        let prefix = if lsargs.recursive {
          path_str.clone() + ":" + consts::NEWLINE
        } else {
          "".to_string()
        };
        let mut totalsize = 0;
        let mut entries: Vec<String> = Vec::new();
        let mut recursive_dirs: Vec<String> = Vec::new();
        for (name, entry) in &dir.entries {
          if entry.is_dir {
            if lsargs.recursive {
              recursive_dirs.push(name.to_string());
            }
            let formatted_name = Colour::Blue.bold().paint(*name).to_string();
            if lsargs.list {
              state.cursor_y += 1;
              totalsize += entry.size;
              entries.push(format!("{}\t{}\t{} {}{}", DIR_PREFIX, entry.get_size(lsargs.human), entry.get_date_str(), formatted_name, consts::NEWLINE));
            } else {
              entries.push(formatted_name);
            }
          } else {
            if lsargs.list {
              state.cursor_y += 1;
              totalsize += entry.size;
              entries.push(format!("{}\t{}\t{} {}{}", FILE_PREFIX, entry.get_size(lsargs.human), entry.get_date_str(), name, consts::NEWLINE));
            } else {
              entries.push(name.to_string());
            }
          }
        }
        if !lsargs.list {
          entries.push(consts::NEWLINE.to_string());
        }
        state.cursor_x = PREFIX.len();
        state.cursor_y += 2;
        if lsargs.recursive {
          for entry in recursive_dirs {
            let mut options = "-R".to_string();
            if lsargs.list { options += "l" }
            if lsargs.human { options += "h" }
            let file = &format!("{}/{}", path_str, entry);
            let mut out = self.ls(state, &format!("ls {} {}", options, file));
            out.truncate(out.len() - PREFIX.len());
            entries.push(out);
          }
        }
        if lsargs.list {
          let totalsize_str = if lsargs.human { utils::human_size(totalsize) } else { format!("{}", totalsize) };
          return format!("{}{}total {}{}{}{}", consts::NEWLINE, prefix, totalsize_str, consts::NEWLINE, &entries.join(""), PREFIX);
        } else {
          return consts::NEWLINE.to_string() + &prefix + &entries.join("\t") + PREFIX;
        }
      } else {
        let file = change.unwrap_or(state.path);
        state.cursor_x = PREFIX.len();
        state.cursor_y += 2;
        let mut filename = file.filename.to_string();
        let mut prefix = format!("{}\t{}\t{} ", FILE_PREFIX, file.size, file.get_date_str());
        if lsargs.directory && (resolved.is_empty() || file.is_dir) {
          filename = Colour::Blue.bold().paint(filename).to_string();
          prefix = format!("{}\t{}\t{} ", DIR_PREFIX, file.size, file.get_date_str());
        }
        if lsargs.list {
          return consts::NEWLINE.to_string() + &prefix + &filename + consts::NEWLINE + PREFIX;
        } else {
          return consts::NEWLINE.to_string() + &filename + consts::NEWLINE + PREFIX;
        }
      }
    }
    state.cursor_y += 2;
    state.cursor_x = PREFIX.len();
    return format!("{}{}: No such file or directory{}{}", consts::NEWLINE, path_str, consts::NEWLINE.to_string(), PREFIX);
  }

  fn cd(&mut self, state: &mut TermState, cmdline: &str) -> String {
    let args: CdArgs = parse_args!(state, CdArgs::try_parse_from(cmdline.split(" ")));
    let path_str = args.dir.unwrap_or("/".to_string());
    let path = state.path.join(path_str.clone());
    let resolved = utils::resolve_path(&path);
    info!("{}", resolved);
    let change = filesystem::ROOT.get_file(resolved);
    if change.is_ok() {
      state.path = change.unwrap();
      let _ = utils::change_url(&("/".to_string() + state.path.url));
      state.cursor_y += 1;
      state.cursor_x = PREFIX.len();
      return consts::NEWLINE.to_string() + PREFIX;
    }
    state.cursor_y += 2;
    state.cursor_x = PREFIX.len();
    return format!("{}{}: No such file or directory{}{}", consts::NEWLINE, path_str, consts::NEWLINE.to_string(), PREFIX);
  }

  fn cat(&mut self, state: &mut TermState, cmdline: &str) -> String {
    let args: CatArgs = parse_args!(state, CatArgs::try_parse_from(cmdline.split(" ")));
    let path_str = args.file;
    let path = state.path.join(path_str.clone());
    info!("{}", path);
    let resolved = utils::resolve_path(&path);
    info!("{}", resolved);
    let change = filesystem::ROOT.get_file(&resolved);
    if change.is_ok() {
      info!("{}", change.clone().unwrap().url);
      let content = change.unwrap().load().unwrap();
      let lines: Vec<&str> = content.lines().collect();
      state.cursor_y += lines.len() + 2;
      state.cursor_x = PREFIX.len();
      return consts::NEWLINE.to_string() + &lines.join(consts::NEWLINE) + consts::NEWLINE + PREFIX;
    }
    state.cursor_y += 2;
    state.cursor_x = PREFIX.len();
    return format!("{}{}: No such file or directory{}{}", consts::NEWLINE, path_str, consts::NEWLINE.to_string(), PREFIX);
  }

  fn pwd(&mut self, state: &mut TermState, _args: &str) -> String {
    return consts::NEWLINE.to_string() + "/" + &state.path.url + consts::NEWLINE + PREFIX;
  }

  fn help(&mut self, _state: &mut TermState, _args: &str) -> String {
    let help = "\
            clear\t\tclear terminal\n\r\
            pwd\t\tprint current directory (or just check URL)\n\r\
            whoami\t\tprint current user\n\r\
            ls\t[PATH]\tlist directory contents\n\r\
            cd\t[DIR]\tchange directory\n\r\
            cat\tFILE\tprint file to stdout\n\r\
            less\tFILE\tview file in screen\n\r\
            echo\tMSG\techo message\n\r\
            help\t\tprint this message\
            ";
    return consts::NEWLINE.to_string() + help + consts::NEWLINE + PREFIX;
  }

  fn parse_less(&mut self, state: &mut TermState, cmdline: &str) -> String {
    let args: LessArgs = parse_args!(state, LessArgs::try_parse_from(cmdline.split(" ")));
    return args.file;
  }

  fn less(&mut self, state: &mut TermState, cmdline: &str) -> (Option<Box<dyn App>>, String) {
    let old_y = state.cursor_y;
    let out = self.parse_less(state, cmdline);
    if old_y != state.cursor_y {
      return (None, out);
    }
    let mut less = Less::new();
    return match less.less(state, &out) {
      Ok(result) => (Some(Box::new(less)), result),
      Err(error) => {
        state.cursor_x = PREFIX.len();
        state.cursor_y += 2;
        (None, error + PREFIX)
      }
    };
  }


  fn command(&mut self, state: &mut TermState, cmdline: &str) -> (Option<Box<dyn App>>, String) {
    let mut history = CMD_HISTORY.lock().unwrap();
    history.push(Box::leak(cmdline.to_owned().into_boxed_str()));
    self.history_index = history.len();
    let mut cmd_args = cmdline.split(" ");
    let cmd = cmd_args.next().unwrap();
    return match cmd {
      "clear" => (None, Shell::clear(state)),
      "pwd" => (None, self.pwd(state, "")),
      "whoami" => (None, self.whoami(state, "")),
      "cd" => (None, self.cd(state, cmdline)),
      "ls" => (None, self.ls(state, cmdline)),
      "cat" => (None, self.cat(state, cmdline)),
      "less" => self.less(state, cmdline),
      "help" => (None, self.help(state, "")),
      "echo" => (None, self.echo(state, "")),
      _ => {
        state.cursor_y += 1;
        state.cursor_x = PREFIX.len();
        (None, format!("{}command not found: {}{}{}", consts::NEWLINE, cmd, consts::NEWLINE, PREFIX))
      }
    };
  }


  fn ansi(&mut self, state: &mut TermState, ansistr: &str) -> String {
    let history = CMD_HISTORY.lock().unwrap();
    match ansistr {
      consts::UP => {
        if self.history_index > 0 {
          self.history_index -= 1;
          let entry = history[self.history_index];
          let out = self.clearline(state) + PREFIX + entry;
          self.input_buffer.clear();
          self.input_buffer.extend(entry.chars());
          state.cursor_x = entry.len() + PREFIX.len();
          return out;
        }
        return "".to_string();
      }
      consts::DOWN => {
        if history.len() != 0 && self.history_index < history.len() - 1 {
          self.history_index += 1;
          let entry = history[self.history_index];
          let out = self.clearline(state) + PREFIX + entry;
          self.input_buffer.clear();
          self.input_buffer.extend(entry.chars());
          state.cursor_x = entry.len() + PREFIX.len();
          return out;
        }
        let out = self.clearline(state) + PREFIX;
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
      consts::INSERT => {
        self.insert = !self.insert;
        self.ansi_buffer.clear();
        self.ansi = false;
      }
      _ => {}
    }
    return "".to_string();
  }
}