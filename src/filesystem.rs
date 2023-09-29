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
    return ron::from_str(ROOT_SERIALIZED).unwrap();
  }

  fn get_file_rec(&self, files: &Vec<&str>) -> Result<&Entry, String> {
    let entry = self.entries.get(files[0]).ok_or("File not found")?;
    let files = &files[1..files.len()].to_vec();
    if files.is_empty() {
      return Ok(entry);
    }
    return entry.get_file_rec(files);
  }

  pub fn get_file(&self, path_str: impl Into<String>) -> Result<&Entry, String> {
    let path = path_str.into();
    let files = utils::resolve_path_files(&path);
    if files.is_empty() {
      return Ok(self);
    }
    return self.get_file_rec(&files);
  }

  pub fn get_size(&self, human: bool) -> String {
    return if human {
      utils::human_size(self.size)
    } else {
      format!("{}", self.size)
    };
  }

  pub fn get_date_str(&self) -> String {
    let datetime = chrono::DateTime::<chrono::Utc>::from_timestamp(
      self.modified as i64, 0,
    ).unwrap();

    return datetime.format("%b %e %H:%M").to_string();
  }

  pub fn join(&self, path_str: impl Into<String>) -> String {
    let path = path_str.into();
    return self.url.to_string() + "/" + &path;
  }

  pub fn load(&self) -> Result<String, String> {
    let url = ROOT_URL.to_string() + self.url;
    info!("loading url: {}", url);
    utils::fetch(url)
  }
}