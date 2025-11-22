use std::collections::HashMap;

use lazy_static::lazy_static;
use log::info;
use serde::Deserialize;

use crate::utils;

include!(concat!(env!("OUT_DIR"), "/root.rs"));

lazy_static! {
    pub static ref ROOT: Entry = Entry::new();
}

#[derive(Deserialize, Debug)]
pub struct Entry {
  pub filename: &'static str,
  pub url: &'static str,
  pub size: u64,
  pub modified: u64,
  pub is_dir: bool,
  pub entries: HashMap<&'static str, Entry>, // only applicable to Dirs
}

const ROOT_URL: &str = "/root/";

impl Entry {
  pub fn new() -> Entry {
    ron::from_str(ROOT_SERIALIZED).unwrap()
  }

  fn get_file_rec(&self, files: &[&str]) -> Result<&Entry, String> {
      let entry = match self.entries.get(files[0]) {
          Some(e) => e,
          None => {
              self.entries.get("old")
                  .ok_or_else(|| "File not found".to_string())?
                  .entries.get(files[0])
                  .ok_or_else(|| "File not found".to_string())?
          }
      };

      let remaining_files = &files[1..];

      if remaining_files.is_empty() {
          Ok(entry)
      } else {
          entry.get_file_rec(remaining_files)
      }
  }

  pub fn get_file(&self, path_str: impl Into<String>) -> Result<&Entry, String> {
    let path = path_str.into();
    let files = utils::resolve_path_files(&path);
    if files.is_empty() {
      return Ok(self);
    }
    self.get_file_rec(&files)
  }

  pub fn get_size(&self, human: bool) -> String {
    if human {
      utils::human_size(self.size)
    } else {
      format!("{}", self.size)
    }
  }

  pub fn get_date_str(&self) -> String {
    let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(
      self.modified as i64, 0,
    ).unwrap();

    datetime.format("%b %e %H:%M").to_string()
  }

  pub fn join(&self, path_str: impl Into<String>) -> String {
    let path = path_str.into();
    if path.starts_with("/") {
      path
    } else {
      self.url.to_string() + "/" + &path
    }
  }

  pub fn load(&self) -> Result<String, String> {
    let url = ROOT_URL.to_string() + self.url;
    info!("loading url: {}", url);
    utils::fetch(url)
  }
}
