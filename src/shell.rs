use log::{info, warn};

use crate::app::App;
use crate::cmds::{self, cmds_init, CMD_HISTORY, COMMANDS};
use crate::termstate::TermState;
use crate::{consts, init, new, prefix, utils, write, write_buf, write_solo, writeln_buf};

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
}

impl Shell {
  pub fn new() -> Self {
    cmds_init();
    Self {
      input_buffer: vec![],
      ansi_buffer: vec![],
      history_index: 0,
      autocomplete_index: 0,
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

  fn autocomplete(&mut self, state: &mut TermState) {}

  fn command(&mut self, state: &mut TermState, cmdline: &str) -> Option<Box<dyn App>> {
    let mut history = CMD_HISTORY.lock().unwrap();
    if history.is_empty() || history[history.len() - 1] != cmdline {
      history.push(Box::leak(cmdline.to_owned().into_boxed_str()));
    }
    self.history_index = history.len();
    drop(history);
    let mut cmd_args = cmdline.split(" ");
    let cmd = cmd_args.next()?;

    return if let Some(command) = COMMANDS.lock().unwrap().get(cmd) {
      command(state, cmdline)
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
      _ => {}
    }
  }
}
