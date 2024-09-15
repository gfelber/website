use std::collections::HashMap;
use std::sync::Mutex;

use ansi_term::Colour;
use clap::{ArgAction, Parser};
use lazy_static::lazy_static;
use log::info;
use macros::{cmds_init, shell_cmd};

use crate::app::App;
use crate::less::Less;
use crate::termstate::TermState;
use crate::{consts, filesystem, utils, write, write_buf, writeln, writeln_buf};

const DIR_PREFIX: &str = "dr-xr-xr-x\t2 root\troot";
const FILE_PREFIX: &str = "-r--r--r--\t1 root\troot";

#[macro_export]
macro_rules! new {
  ($state:expr) => {{
    $state.cursor_x = 0;
    writeln_buf!($state, "");
  }};
}

#[macro_export]
macro_rules! prefix {
  ($state:expr) => {{
    $state.cursor_x = consts::PREFIX.len();
    write!("{}", consts::PREFIX);
  }};
}

#[macro_export]
macro_rules! init {
  ($state:expr) => {{
    new!($state);
    prefix!($state);
  }};
}

#[macro_export]
macro_rules! write_solo {
  ($state:expr, $out:expr) => {{
    new!($state);
    write_buf!("{}", $out);
    init!($state);
  }};
}

#[macro_export]
macro_rules! parse_args {
  ($state:expr, $e:expr, $ret:expr) => {
    match $e {
      Ok(args) => args,
      Err(error) => {
        let error_str = error.to_string();
        let lines: Vec<&str> = error_str.lines().collect();
        $state.cursor_y += lines.len() + 2;
        write_solo!($state, lines.join(consts::NEWLINE));
        return $ret;
      }
    }
  };
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

type CommandFn = fn(&mut TermState, &str) -> Option<Box<dyn App>>;

lazy_static! {
  pub static ref COMMANDS: Mutex<HashMap<&'static str, CommandFn>> = Mutex::new(HashMap::new());
  pub static ref CMD_HISTORY: Mutex<Vec<&'static str>> = Mutex::new(vec![]);
}

#[shell_cmd(COMMANDS)]
pub fn echo(state: &mut TermState, args: &str) -> Option<Box<dyn App>> {
  let mut cmd_args = args.splitn(2, ' ');
  _ = cmd_args.next();
  write_solo!(state, cmd_args.next().unwrap_or(""));
  None
}

#[shell_cmd(COMMANDS)]
pub fn whereis(state: &mut TermState, _args: &str) -> Option<Box<dyn App>> {
  write_solo!(state, "https://github.com/gfelber/website");
  None
}

#[shell_cmd(COMMANDS)]
pub fn whoami(state: &mut TermState, _args: &str) -> Option<Box<dyn App>> {
  write_solo!(state, "gfelber/0x6fe1be2 (https://github.com/gfelber)");
  None
}

#[shell_cmd(COMMANDS)]
fn history(state: &mut TermState, _args: &str) -> Option<Box<dyn App>> {
  let history = CMD_HISTORY.lock().unwrap();
  new!(state);
  for (index, cmd) in history.iter().enumerate() {
    writeln!(state, "{:-4} {}", index, cmd);
  }
  prefix!(state);
  None
}

#[shell_cmd(COMMANDS)]
pub fn ls(state: &mut TermState, cmdline: &str) -> Option<Box<dyn App>> {
  let out = ls_rec(state, cmdline);
  write!("{}", out);
  // only is empty if error was encountered
  if !out.is_empty() {
    prefix!(state);
  }
  None
}

pub fn ls_rec(state: &mut TermState, cmdline: &str) -> String {
  let lsargs = parse_args!(
    state,
    LsArgs::try_parse_from(cmdline.split(" ")),
    "".to_string()
  );
  let path_str = lsargs.file.unwrap_or(".".to_string());
  let path = state.path.join(path_str.clone());
  let resolved = utils::resolve_path(&path);
  info!("{}", resolved);
  let change = filesystem::ROOT.get_file(&resolved);
  if resolved.is_empty() || change.is_ok() {
    return if !lsargs.directory && (resolved.is_empty() || change.clone().unwrap().is_dir) {
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
            entries.push(format!(
              "{}\t{}\t{} {}{}",
              DIR_PREFIX,
              entry.get_size(lsargs.human),
              entry.get_date_str(),
              formatted_name,
              consts::NEWLINE
            ));
          } else {
            entries.push(formatted_name);
          }
        } else {
          if lsargs.list {
            state.cursor_y += 1;
            totalsize += entry.size;
            entries.push(format!(
              "{}\t{}\t{} {}{}",
              FILE_PREFIX,
              entry.get_size(lsargs.human),
              entry.get_date_str(),
              name,
              consts::NEWLINE
            ));
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
          if lsargs.list {
            options += "l"
          }
          if lsargs.human {
            options += "h"
          }
          let file = &format!("{}/{}", path_str, entry);
          let out = ls_rec(state, &format!("ls {} {}", options, file));
          entries.push(out);
        }
      }
      if lsargs.list {
        let totalsize_str = if lsargs.human {
          utils::human_size(totalsize)
        } else {
          format!("{}", totalsize)
        };
        format!(
          "{}{}total {}{}{}",
          consts::NEWLINE,
          prefix,
          totalsize_str,
          consts::NEWLINE,
          &entries.join("")
        )
      } else {
        format!("{}{}{}", consts::NEWLINE, prefix, &entries.join("\t"))
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
      if lsargs.list {
        format!(
          "{}{}{}{}",
          consts::NEWLINE,
          prefix,
          filename,
          consts::NEWLINE
        )
      } else {
        format!("{}{}{}", consts::NEWLINE, filename, consts::NEWLINE)
      }
    };
  }
  state.cursor_y += 2;
  format!(
    "{}{}: No such file or directory{}",
    consts::NEWLINE,
    path_str,
    consts::NEWLINE
  )
}

#[shell_cmd(COMMANDS)]
pub fn cd(state: &mut TermState, cmdline: &str) -> Option<Box<dyn App>> {
  let args: CdArgs = parse_args!(state, CdArgs::try_parse_from(cmdline.split(" ")), None);
  let path_str = args.dir.unwrap_or("/".to_string());
  let path = state.path.join(path_str.clone());
  let resolved = utils::resolve_path(&path);
  info!("{}", resolved);
  let change = filesystem::ROOT.get_file(resolved);
  if change.is_ok() {
    let dir = change.unwrap();
    if !dir.is_dir {
      write_solo!(state, format!("can't cd to {}: Not a directory", path_str));
      return None;
    }
    state.path = dir;
    let _ = utils::change_url(&("/".to_string() + state.path.url));
    init!(state);
  } else {
    write_solo!(state, format!("{}: No such directory", path_str));
  }
  None
}

#[shell_cmd(COMMANDS)]
pub fn cat(state: &mut TermState, cmdline: &str) -> Option<Box<dyn App>> {
  let args: CatArgs = parse_args!(state, CatArgs::try_parse_from(cmdline.split(" ")), None);
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
      return None;
    }
    info!("{}", file.url);
    let content = file.load().unwrap();
    let lines: Vec<&str> = content.lines().collect();
    state.cursor_y += lines.len() + 2;
    state.cursor_x = consts::PREFIX.len();
    new!(state);
    for line in lines {
      writeln!(state, "{}", line);
    }
    prefix!(state);
  } else {
    write_solo!(state, format!("{}: No such file", path_str));
  }
  None
}

#[shell_cmd(COMMANDS)]
pub fn pwd(state: &mut TermState, _args: &str) -> Option<Box<dyn App>> {
  write_solo!(state, "/".to_string() + &state.path.url);
  None
}

#[shell_cmd(COMMANDS)]
pub fn help(state: &mut TermState, _args: &str) -> Option<Box<dyn App>> {
  // TODO: generate from macro data
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
          history\t\tprint cmd history\n\r\
          help\t\tprint this message\
          ";
  write_solo!(state, help);
  None
}

#[shell_cmd(COMMANDS)]
pub fn less(state: &mut TermState, cmdline: &str) -> Option<Box<dyn App>> {
  let args: LessArgs = parse_args!(state, LessArgs::try_parse_from(cmdline.split(" ")), None);
  let mut less = Less::new();
  match less.less(state, &args.file) {
    Ok(()) => {
      let app_box: Box<dyn App> = Box::new(less);
      Some(app_box)
    }
    Err(error) => {
      write_solo!(state, error);
      None
    }
  }
}

#[cmds_init]
pub fn cmds_init() {}
