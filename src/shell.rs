use std::sync::Mutex;

use ansi_term::Colour;
use clap::{ArgAction, Parser};
use lazy_static::lazy_static;
use log::{info, warn};

use crate::{clear, consts, filesystem, utils, write, writeln, write_buf, writeln_buf};
use crate::app::App;
use crate::less::Less;
use crate::termstate::TermState;

const PREFIX: &str = "$ ";
const DIR_PREFIX: &str = "dr-xr-xr-x\t2 root\troot";
const FILE_PREFIX: &str = "-r--r--r--\t1 root\troot";

macro_rules! new {
  ($state:expr) => {{
    $state.cursor_x = 0;
    writeln_buf!($state, "");
  }};
}

macro_rules! prefix {
  ($state:expr) => {{
    $state.cursor_x += PREFIX.len();
    write!("{}", PREFIX);
  }};
}
macro_rules! init {
  ($state:expr) => {{
    new!($state);
    prefix!($state);
  }};
}
macro_rules! write_solo {
  ($state:expr, $out:expr) => {{
    new!($state);
    write_buf!("{}", $out);
    init!($state);
  }};
}

macro_rules! parse_args {
  ($state:expr, $e:expr, $ret:expr) => {
    match $e{
      Ok(args) => {args}
      Err(error) => {
        let error_str = error.to_string();
        let lines: Vec<&str> = error_str.lines().collect();
        $state.cursor_x = PREFIX.len();
        $state.cursor_y += lines.len() + 2;
        write_solo!($state, lines.join(consts::NEWLINE));
        return $ret;
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
  dir: Option<String>,
}

#[derive(Parser)]
#[command(about = "print file to stdout")]
struct CatArgs {
  #[arg(help = "file to print")]
  file: String,
}

#[derive(Parser)]
#[command(about = "view file inside screen")]
struct LessArgs {
  #[arg(help = "file to view")]
  file: String,
}

lazy_static! {
    static ref CMD_HISTORY: Mutex<Vec<&'static str>> = Mutex::new(vec![]);
}

pub struct Shell {
  input_buffer: Vec<char>,
  ansi_buffer: Vec<char>,
  history_index: usize,
  autocomplete_index: usize,
  ansi: bool,
  insert: bool,
}

impl App for Shell {
  fn readchar(&mut self, state: &mut TermState, input: char) -> Option<Box<dyn App>> {
    if self.ansi {
      self.ansi_buffer.push(input);
      let ansistr: String = self.ansi_buffer.iter().collect();
      let mut hex = "".to_string();
      for byt in ansistr.as_bytes() {
        hex += &format!("{:02X}", byt);
      }
      info!("{}", hex);
      self.ansi(state, &ansistr);
      return None;
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
        self.clearline(state);
        self.input_buffer.clear();
        None
      }
      // clear
      '\x0c' => {
        self.input_buffer.clear();
        Shell::clear(state);
        None
      }
      // return key
      '\x7f' => {
        if self.input_buffer.is_empty() {
          return None;
        }
        let cursor_x = state.cursor_x - (PREFIX.len() + 1);
        info!("{}/{}", cursor_x, self.input_buffer.len());
        let left = consts::LEFT.repeat(self.input_buffer.len() - cursor_x);
        self.clearline(state);
        self.input_buffer.remove(cursor_x);
        let inputstr: String = self.input_buffer.iter().collect();
        write!("{} {}", inputstr, left);
        state.cursor_x = cursor_x + PREFIX.len();
        None
      }
      // ansi
      '\x1b' => {
        self.ansi = true;
        self.ansi_buffer.push(input);
        None
      }
      '\t' => {
        self.autocomplete(state);
        None
      }
      // only printable characters
      c if c >= ' ' => {
        if state.cursor_x < self.input_buffer.len() + PREFIX.len() {
          if self.insert {
            self.input_buffer[state.cursor_x - PREFIX.len()] = input;
          } else {
            self.input_buffer.insert(state.cursor_x - PREFIX.len(), input);
            let new_x = state.cursor_x + 1;
            let input_str: String = self.input_buffer.iter().collect();
            let left = consts::LEFT.repeat(self.input_buffer.len() - (new_x - PREFIX.len()));
            self.clearline(state);
            write!("{}{}", input_str, left);
            state.cursor_x = new_x;
            return None;
          }
        } else if state.cursor_x < state.width - 1 {
          state.cursor_x += 1;
          self.input_buffer.push(input);
        } else {
          info!("reached EOL");
          return None;
        }
        write!("{}", input);
        None
      }
      _ => {
        warn!("character not supported: {:02x}", input as u32);
        None
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
      autocomplete_index: 0,
      ansi: false,
      insert: false,
    }
  }

  pub fn clear(state: &mut TermState) {
    clear!(state);
    prefix!(state);
  }

  fn clearline(&mut self, state: &mut TermState) {
    let right: String = consts::RIGHT.repeat(self.input_buffer.len() - (state.cursor_x - PREFIX.len()));
    let clear: String = consts::RETURN.repeat(self.input_buffer.len());
    state.cursor_x = PREFIX.len();
    write_buf!("{}{}", right, clear);
  }

  fn autocomplete(&mut self, state: &mut TermState) {}

  fn echo(&mut self, state: &mut TermState, args: &str) {
    write_solo!(state, args);
  }

  fn whereis(&mut self, state: &mut TermState, _args: &str) {
    write_solo!(state, "https://github.com/gfelber/website");
  }

  fn whoami(&mut self, state: &mut TermState, _args: &str) {
    write_solo!(state, "gfelber (0x6fe1be2)");
  }

  fn history(&mut self, state: &mut TermState, _args: &str) {
    let history = CMD_HISTORY.lock().unwrap();
    new!(state);
    for (index, cmd) in history.iter().enumerate() {
      writeln!(state, "{:-4} {}", index, cmd);
    }
    prefix!(state);
  }

  fn ls(&mut self, state: &mut TermState, cmdline: &str) {
    write_buf!("{}", self.ls_rec(state, cmdline));
    state.cursor_x = 0;
    prefix!(state);
  }
  fn ls_rec(&mut self, state: &mut TermState, cmdline: &str) -> String {
    let lsargs = parse_args!(state, LsArgs::try_parse_from(cmdline.split(" ")), "".to_string());
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
        state.cursor_y += 2;
        if lsargs.recursive {
          for entry in recursive_dirs {
            let mut options = "-R".to_string();
            if lsargs.list { options += "l" }
            if lsargs.human { options += "h" }
            let file = &format!("{}/{}", path_str, entry);
            let out = self.ls_rec(state, &format!("ls {} {}", options, file));
            entries.push(out);
          }
        }
        return if lsargs.list {
          let totalsize_str = if lsargs.human { utils::human_size(totalsize) } else { format!("{}", totalsize) };
          format!("{}{}total {}{}{}", consts::NEWLINE, prefix, totalsize_str, consts::NEWLINE, &entries.join(""))
        } else {
          consts::NEWLINE.to_string() + &prefix + &entries.join("\t")
        }
      } else {
        let file = change.unwrap_or(state.path);
        state.cursor_y += 2;
        let mut filename = file.filename.to_string();
        let mut prefix = format!("{}\t{}\t{} ", FILE_PREFIX, file.size, file.get_date_str());
        if lsargs.directory && (resolved.is_empty() || file.is_dir) {
          filename = Colour::Blue.bold().paint(filename).to_string();
          prefix = format!("{}\t{}\t{} ", DIR_PREFIX, file.size, file.get_date_str());
        }
        return if lsargs.list {
          consts::NEWLINE.to_string() + &prefix + &filename + consts::NEWLINE
        } else {
          consts::NEWLINE.to_string() + &filename + consts::NEWLINE
        }
      }
    }
    state.cursor_y += 2;
    return format!("{}{}: No such file or directory{}", consts::NEWLINE, path_str, consts::NEWLINE.to_string());
  }

  fn cd(&mut self, state: &mut TermState, cmdline: &str) {
    let args: CdArgs = parse_args!(state, CdArgs::try_parse_from(cmdline.split(" ")), ());
    let path_str = args.dir.unwrap_or("/".to_string());
    let path = state.path.join(path_str.clone());
    let resolved = utils::resolve_path(&path);
    info!("{}", resolved);
    let change = filesystem::ROOT.get_file(resolved);
    if change.is_ok()  {
      let dir = change.unwrap();
      if !dir.is_dir{
        write_solo!(state, format!("can't cd to {}: Not a directory", path_str));
        return;
      }
      state.path = dir;
      let _ = utils::change_url(&("/".to_string() + state.path.url));
      init!(state);
    } else {
      write_solo!(state, format!("{}: No such directory", path_str));
    }
  }

  fn cat(&mut self, state: &mut TermState, cmdline: &str) {
    let args: CatArgs = parse_args!(state, CatArgs::try_parse_from(cmdline.split(" ")), ());
    let path_str = args.file;
    let path = state.path.join(path_str.clone());
    info!("{}", path);
    let resolved = utils::resolve_path(&path);
    info!("{}", resolved);
    let change = filesystem::ROOT.get_file(&resolved);
    if change.is_ok() {
      let file = change.unwrap();
      if file.is_dir {
        write_solo!(state, format!("read error: {} Is a directory", path_str));
        return;
      }
      info!("{}", file.url);
      let content = file.load().unwrap();
      let lines: Vec<&str> = content.lines().collect();
      state.cursor_y += lines.len() + 2;
      state.cursor_x = PREFIX.len();
      new!(state);
      for line in lines {
        writeln!(state, "{}", line);
      }
      prefix!(state);
    } else {
      write_solo!(state, format!("{}: No such file", path_str));
    }
  }

  fn pwd(&mut self, state: &mut TermState, _args: &str) {
    write_solo!(state, "/".to_string() + &state.path.url);
  }

  fn help(&mut self, state: &mut TermState, _args: &str) {
    let help = "\
            clear\t\tclear terminal\n\r\
            pwd\t\tprint current directory (or just check URL)\n\r\
            whoami\t\tprint current user\n\r\
            whereis\t\tLocate where stuff is\n\r\
            ls\t[PATH]\tlist directory contents\n\r\
            cd\t[DIR]\tchange directory\n\r\
            cat\tFILE\tprint file to stdout\n\r\
            less\tFILE\tview file in screen\n\r\
            echo\tMSG\techo message\n\r\
            history\t\tprint cmd history\
            help\t\tprint this message\
            ";
    write_solo!(state, help);
  }

  fn less(&mut self, state: &mut TermState, cmdline: &str) -> Option<Box<dyn App>> {
    let args: LessArgs = parse_args!(state, LessArgs::try_parse_from(cmdline.split(" ")), None);
    let mut less = Less::new();
    return match less.less(state, &args.file) {
      Ok(()) => Some(Box::new(less)),
      Err(error) => {
        write_solo!(state, error);
        None
      }
    };
  }


  fn command(&mut self, state: &mut TermState, cmdline: &str) -> Option<Box<dyn App>> {
    let mut history = CMD_HISTORY.lock().unwrap();
    if history.is_empty() || history[history.len() - 1] != cmdline {
      history.push(Box::leak(cmdline.to_owned().into_boxed_str()));
    }
    self.history_index = history.len();
    drop(history);
    let mut cmd_args = cmdline.split(" ");
    let cmd = cmd_args.next().unwrap();
    match cmd {
      "clear" => Shell::clear(state),
      "pwd" => self.pwd(state, ""),
      "whoami" => self.whoami(state, ""),
      "cd" => self.cd(state, cmdline),
      "ls" => self.ls(state, cmdline),
      "cat" => self.cat(state, cmdline),
      "whereis" => self.whereis(state, cmdline),
      "less" => {
        return self.less(state, cmdline);
      }
      "echo" => self.echo(state, cmd_args.remainder().unwrap_or("")),
      "help" => self.help(state, ""),
      "history" => self.history(state, ""),
      _ => {
        state.cursor_y += 1;
        state.cursor_x = PREFIX.len();
        write_solo!(state, format!("command not found: {}", cmd));
      }
    };
    return None;
  }


  fn ansi_clear(&mut self) {
    self.ansi_buffer.clear();
    self.ansi = false;
  }
  fn ansi(&mut self, state: &mut TermState, ansistr: &str) {
    let history = CMD_HISTORY.lock().unwrap();
    match ansistr {
      consts::UP => {
        self.ansi_clear();
        if self.history_index > 0 {
          self.history_index -= 1;
          let entry = history[self.history_index];
          self.clearline(state);
          self.input_buffer.clear();
          self.input_buffer.extend(entry.chars());
          state.cursor_x = entry.len() + PREFIX.len();
          write!("{}", entry.to_string());
        }
      }
      consts::DOWN => {
        self.ansi_clear();
        if history.len() != 0 && self.history_index < history.len() - 1 {
          self.history_index += 1;
          let entry = history[self.history_index];
          self.clearline(state);
          self.input_buffer.clear();
          self.input_buffer.extend(entry.chars());
          state.cursor_x = entry.len() + PREFIX.len();
          write!("{}", entry);
        } else if self.history_index < history.len() {
          self.history_index += 1
        }
        self.clearline(state);
        write!("");
        self.input_buffer.clear();
        state.cursor_x = PREFIX.len();
      }
      consts::RIGHT => {
        self.ansi_clear();
        if state.cursor_x < self.input_buffer.len() + PREFIX.len() {
          state.cursor_x += 1;
          write!("{}", consts::RIGHT);
        }
      }
      consts::LEFT => {
        self.ansi_clear();
        if state.cursor_x > PREFIX.len() {
          state.cursor_x -= 1;
          write!("{}", consts::LEFT);
        }
      }
      consts::PAGE_START => {
        self.ansi_clear();
        let repeat = state.cursor_x - PREFIX.len();
        state.cursor_x = PREFIX.len();
        write!("{}", consts::LEFT.repeat(repeat));
      }
      consts::PAGE_END => {
        self.ansi_clear();
        let repeat = self.input_buffer.len() - state.cursor_x - PREFIX.len();
        state.cursor_x = self.input_buffer.len() + PREFIX.len();
        write!("{}", consts::RIGHT.repeat(repeat));
      }
      consts::INSERT => {
        self.ansi_clear();
        self.insert = !self.insert;
      }
      _ => {}
    }
  }
}