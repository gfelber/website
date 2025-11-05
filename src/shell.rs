use std::cmp::max;

use log::{info, warn};

use crate::app::App;
use crate::cmds::{self, CMD_HISTORY, COMMANDS, MobileType};
use crate::termstate::TermState;
use crate::utils::longest_common_prefix;
use crate::{
  consts, filesystem, init, new, prefix, utils, write, write_buf, write_solo, writeln_buf,
};

pub struct Shell {
  input_buffer: Vec<char>,
  ansi_buffer: Vec<char>,
  history_index: usize,
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
      if self.ansi && ansistr.len() > 3 && input == '\x7e' {
        warn!("invalid ansi sequence");
        self.ansi_clear();
      }
      return None;
    }
    match input {
      '\r' | '\n' => {
        let cmd: String = self.input_buffer.iter().collect();
        info!("{}", cmd);
        self.input_buffer.clear();
        state.cursor_x = consts::PREFIX.len();
        self.command(state, &cmd)
      }
      // clear line
      '\x15' => {
        self.clearline(state);
        self.input_buffer.clear();
        write!("");
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
        let cursor_x = state.cursor_x - (consts::PREFIX.len() + 1);
        info!("{}/{}", cursor_x, self.input_buffer.len());
        let left = consts::LEFT.repeat(self.input_buffer.len() - cursor_x);
        self.clearline(state);
        self.input_buffer.remove(cursor_x);
        let inputstr: String = self.input_buffer.iter().collect();
        write!("{} {}", inputstr, left);
        state.cursor_x = cursor_x + consts::PREFIX.len();
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
        if state.cursor_x < self.input_buffer.len() + consts::PREFIX.len() {
          if self.insert {
            self.input_buffer[state.cursor_x - consts::PREFIX.len()] = input;
          } else {
            self
              .input_buffer
              .insert(state.cursor_x - consts::PREFIX.len(), input);
            let new_x = state.cursor_x + 1;
            let input_str: String = self.input_buffer.iter().collect();
            let left =
              consts::LEFT.repeat(self.input_buffer.len() - (new_x - consts::PREFIX.len()));
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
    }
  }

  fn scroll(&mut self, _state: &mut TermState, _lines: i32) {}

  fn autocomplete(&self, state: &TermState) -> Vec<String> {
    self.get_autocomplete_options(state, true)
  }
}



impl Shell {
  pub fn new() -> Self {
    let history = CMD_HISTORY.lock().unwrap();
    Self {
      input_buffer: vec![],
      ansi_buffer: vec![],
      history_index: max(history.len(), 1) - 1,
      ansi: false,
      insert: false,
    }
  }

  pub fn clear(state: &mut TermState) {
    cmds::clear(state, "");
  }

  fn clearline(&self, state: &mut TermState) {
    let right: String =
      consts::RIGHT.repeat(self.input_buffer.len() - (state.cursor_x - consts::PREFIX.len()));
    let clear: String = consts::RETURN.repeat(self.input_buffer.len());
    state.cursor_x = consts::PREFIX.len();
    write_buf!("{}{}", right, clear);
  }

  pub fn get_autocomplete_options(&self, state: &TermState, mobile: bool) -> Vec<String> {
    let inputstr: String = self.input_buffer.iter().collect();

    // Command autocompletion (no space in input)
    if !inputstr.contains(' ') {
      let cmds: Vec<_> = COMMANDS.lock().unwrap().keys().cloned().collect();
      let filtered_cmds: Vec<_> = cmds
        .iter()
        .filter(|cmd| cmd.starts_with(&inputstr) && (!mobile || COMMANDS.lock().unwrap()[**cmd].mobile != MobileType::NotMobile))
        .map(|cmd| cmd.to_string())
        .collect();
      return filtered_cmds;
    }

    let cmd = inputstr.split(' ').next().unwrap();
    let cmd_info = {
      let commands = COMMANDS.lock().unwrap();
      commands.get(cmd).cloned()
    };

    if cmd_info.is_none() {
      return vec![];
    }
    let cmd_info = cmd_info.unwrap();

    if mobile && cmd_info.mobile == MobileType::Mobile {
      return vec![];
    }

    if inputstr.split(' ').count() - 1 > 1 {
      return vec![];
    }

    // File/directory autocompletion
    let wsi = match inputstr.rfind(' ') {
      Some(idx) => idx,
      None => return vec![],
    };
    let search = &inputstr[wsi + 1..];

    let mut search_vals = search.rsplitn(2, '/');
    let filename = search_vals.next().unwrap();
    let crnt_path = search_vals.next().unwrap_or("");

    let path = if search.starts_with('/') {
      crnt_path
    } else if search.contains('/') {
      &state.path.join(crnt_path)
    } else {
      state.path.filename
    };

    let resolved = utils::resolve_path(path);
    let change = filesystem::ROOT.get_file(&resolved);
    if change.is_err() || !change.clone().unwrap().is_dir {
      return vec![];
    }

    let dir = if resolved.is_empty() {
      &filesystem::ROOT
    } else {
      change.unwrap()
    };
    let entries: Vec<_> = dir.entries.keys().cloned().collect();

    let filtered_entries: Vec<_> = entries
      .iter()
      .filter(|entry| entry.starts_with(&filename))
      .map(|entry| {
        if dir.get_file(entry.to_string()).unwrap().is_dir {
          format!("{}/", entry)
        } else {
          entry.to_string()
        }
      })
      .collect();

    filtered_entries
  }

  fn autocomplete(&mut self, state: &mut TermState) {
    let mut inputstr: String = self.input_buffer.iter().collect();
    if !inputstr.contains(' ') {
      let cmds: Vec<_> = COMMANDS.lock().unwrap().keys().cloned().collect();
      let filtered_cmds: Vec<_> = cmds
        .iter()
        .filter(|cmd| cmd.starts_with(&inputstr))
        .map(|cmd| cmd.to_owned())
        .collect();
      return if filtered_cmds.is_empty() {
      } else if filtered_cmds.len() == 1 {
        let cmd = format!("{} ", filtered_cmds.first().unwrap());
        self.clearline(state);
        write!("{}", cmd);
        state.cursor_x += cmd.len();
        self.input_buffer = cmd.chars().collect();
      } else {
        let prefix = longest_common_prefix(filtered_cmds.clone());
        info!("common prefix: {}", prefix);
        write_solo!(state, filtered_cmds.join("\t"));
        self
          .input_buffer
          .append(&mut prefix.trim_start_matches(&inputstr).chars().collect());
        inputstr = self.input_buffer.iter().collect();
        state.cursor_x += inputstr.len();
        write!("{}", inputstr);
      };
    }
    let wsi = inputstr.rfind(' ').unwrap();
    let search = &inputstr[wsi + 1..];

    let mut search_vals = search.rsplitn(2, '/');
    let filename = search_vals.next().unwrap();
    let crnt_path = search_vals.next().unwrap_or("");

    info!("current path: {}", crnt_path);
    info!("filename: {}", filename);

    let path = if search.starts_with('/') {
      crnt_path
    } else if search.contains('/') {
      &state.path.join(crnt_path)
    } else {
      state.path.url
    };

    info!("autocomplete path: {}", path);

    let resolved = utils::resolve_path(path);
    let change = filesystem::ROOT.get_file(&resolved);
    if change.is_err() || !change.clone().unwrap().is_dir {
      return;
    }

    let dir = if resolved.is_empty() {
      &filesystem::ROOT
    } else {
      change.unwrap()
    };
    let entries: Vec<_> = dir.entries.keys().cloned().collect();

    let filtered_entries: Vec<_> = entries
      .iter()
      .filter(|entry| entry.starts_with(&filename))
      .map(|entry| entry.to_owned())
      .collect();

    return if filtered_entries.is_empty() {
    } else if filtered_entries.len() == 1 {
      let entry_name = filtered_entries.first().unwrap();
      let entry = if dir.get_file(entry_name.to_string()).unwrap().is_dir {
        format!("{}/", entry_name)
      } else {
        format!("{} ", entry_name)
      };
      info!("autcomplete entry: {}", entry);
      let clear: String = consts::RETURN.repeat(filename.len());
      write!("{}{}", clear, entry);
      state.cursor_x += entry.len() - filename.len();
      self
        .input_buffer
        .truncate(self.input_buffer.len().saturating_sub(filename.len()));
      let mut entry_chars: Vec<char> = entry.chars().collect();
      self.input_buffer.append(&mut entry_chars);
    } else {
      write_solo!(
        state,
        filtered_entries
          .clone()
          .into_iter()
          .map(|x| {
            return if dir.get_file(x.to_string()).unwrap().is_dir {
              format!("{}/", x)
            } else {
              x.to_string()
            };
          })
          .collect::<Vec<_>>()
          .join("\t")
      );
      let prefix = longest_common_prefix(filtered_entries);
      info!("common prefix: {}", prefix);
      self
        .input_buffer
        .append(&mut prefix.trim_start_matches(filename).chars().collect());
      inputstr = self.input_buffer.iter().collect();
      state.cursor_x += inputstr.len();
      write!("{}", inputstr);
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
    let cmd = cmd_args.next()?;

    let cmd_info = {
      let commands = COMMANDS.lock().unwrap();
      commands.get(cmd).cloned()
    };

    return if let Some(cmd_info) = cmd_info {
      (cmd_info.func)(state, cmdline.trim_end_matches(' '))
    } else {
      state.cursor_y += 1;
      state.cursor_x = consts::PREFIX.len();
      write_solo!(state, format!("command not found: {}, try using help", cmd));
      None
    };
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
          state.cursor_x = entry.len() + consts::PREFIX.len();
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
          state.cursor_x = entry.len() + consts::PREFIX.len();
          write!("{}", entry);
        } else if self.history_index < history.len() {
          self.history_index += 1
        }
        self.clearline(state);
        write!("");
        self.input_buffer.clear();
        state.cursor_x = consts::PREFIX.len();
      }
      consts::RIGHT => {
        self.ansi_clear();
        if state.cursor_x < self.input_buffer.len() + consts::PREFIX.len() {
          state.cursor_x += 1;
          write!("{}", consts::RIGHT);
        }
      }
      consts::LEFT => {
        self.ansi_clear();
        if state.cursor_x > consts::PREFIX.len() {
          state.cursor_x -= 1;
          write!("{}", consts::LEFT);
        }
      }
      consts::PAGE_START => {
        self.ansi_clear();
        let repeat = state.cursor_x - consts::PREFIX.len();
        state.cursor_x = consts::PREFIX.len();
        write!("{}", consts::LEFT.repeat(repeat));
      }
      consts::PAGE_END => {
        self.ansi_clear();
        let repeat = self.input_buffer.len() + consts::PREFIX.len() - state.cursor_x;
        state.cursor_x = self.input_buffer.len() + consts::PREFIX.len();
        write!("{}", consts::RIGHT.repeat(repeat));
      }
      consts::INSERT => {
        self.ansi_clear();
        self.insert = !self.insert;
      }
      consts::F1 | consts::F2 | consts::F3 | consts::F4 => {
        self.ansi_clear();
      }
      _ => {}
    }
  }
}
