use ansi_term::{Colour, Style};
use log::info;

use crate::{consts, filesystem, utils};
use crate::app::App;
use crate::consts::NEWLINE;
use crate::shell::Shell;
use crate::termstate::TermState;

pub struct Less {
  ansi_buffer: Vec<char>,
  lines: Vec<&'static str>,
  line: usize,
  ansi: bool,
}

impl App for Less {
  fn readchar(&mut self, state: &mut TermState, input: char) -> (Option<Box<dyn App>>, String) {
    if self.ansi {
      self.ansi_buffer.push(input);
      let ansistr: String = self.ansi_buffer.iter().collect();
      return (None, self.ansi(state, &ansistr));
    }
    return match input {
      // ansi
      '\x1b' => {
        self.ansi = true;
        self.ansi_buffer.push(input);
        (None, "".to_string())
      }
      // quit
      'q' => {
        let _ = utils::change_url(&("/".to_string() + state.path.url));
        (Some(Box::new(Shell::new())), Shell::clear(state))
      }
      // top
      'g' => {
        (None, self.less_from(state, 0))
      }
      // bottom
      'G' => {
        (None, self.less_from(state, usize::MAX))
      }
      _ => {
        (None, "".to_string())
      }
    };
  }
}

impl Less {
  pub fn new() -> Self {
    Self {
      ansi_buffer: vec![],
      lines: vec![],
      line: 0,
      ansi: false,
    }
  }
  fn less_from(&mut self, state: &mut TermState, mut n: usize) -> String {
    let lines_len = self.lines.len();
    let bound: usize = if lines_len > state.height { lines_len - state.height } else { 0 };
    info!("{}", format!("{} {}", n, bound));
    n = if n < bound { n } else { bound };
    info!("{}", format!("{}", n));
    self.line = n;
    let m: usize = if n + state.height - 1 < lines_len { n + state.height - 1 } else { lines_len };
    let head: Vec<&str> = self.lines[n..m].to_vec();
    let padding = state.height - head.len();
    let suffix = if n == bound {
      Style::new().on(Colour::RGB(234, 255, 229))
        .fg(Colour::Black)
        .paint("(END)").to_string()
    } else {
      ":".to_string()
    };
    return consts::NEWLINE.repeat(padding) + &head.join(NEWLINE) + consts::NEWLINE + &suffix;
  }

  pub fn less(&mut self, state: &mut TermState, path_str: &str) -> Result<String, String> {
    let path = state.path.join(path_str);
    info!("{}", path);
    let resolved = utils::resolve_path(&path);
    info!("{}", resolved);
    let change = filesystem::ROOT.get_file(resolved.clone());
    if change.is_ok() {
      let file = change.unwrap();
      let _ = utils::change_url(&("/".to_string() + file.url));
      info!("{}", file.url);
      let content = Box::leak(Box::new(file.load().unwrap()));
      self.lines = content.lines().collect();
      return Ok(self.less_from(state, 0));
    }

    return Err(format!("{}{}: No such file or directory{}", consts::NEWLINE, path_str, consts::NEWLINE.to_string()));
  }

  fn ansi(&mut self, state: &mut TermState, ansistr: &str) -> String {
    return match ansistr {
      consts::UP => {
        self.ansi = false;
        self.ansi_buffer.clear();
        self.less_from(state, if self.line > 0 { self.line - 1 } else { 0 })
      }
      consts::DOWN => {
        self.ansi = false;
        self.ansi_buffer.clear();
        self.less_from(state, self.line + 1)
      }
      consts::PAGE_UP => {
        self.ansi = false;
        self.ansi_buffer.clear();
        self.less_from(state, if self.line > state.height { self.line - state.height } else { 0 })
      }
      consts::PAGE_DOWN => {
        self.ansi = false;
        self.ansi_buffer.clear();
        self.less_from(state, self.line + state.height)
      }
      consts::PAGE_START => {
        self.ansi = false;
        self.ansi_buffer.clear();
        self.less_from(state, 0)
      }
      consts::PAGE_END => {
        self.ansi = false;
        self.ansi_buffer.clear();
        self.less_from(state, usize::MAX)
      }
      consts::RIGHT | consts::LEFT => {
        self.ansi = false;
        self.ansi_buffer.clear();
        "".to_string()
      }
      _ => "".to_string()
    };
  }
}