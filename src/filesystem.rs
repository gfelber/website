use std::collections::HashMap;
use log::info;
use serde::Deserialize;
use crate::utils;
include!(concat!(env!("OUT_DIR"), "/root.rs"));

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
impl Entry{
  pub fn new() -> Entry{
    return ron::from_str(ROOT).unwrap();
  }

  fn get_file_rec(&self, files: &Vec<&str>) -> Result<&Entry, String> {
    let entry = self.entries.get(files[0]).ok_or("File not found")?;
    let files = &files[1..files.len()].to_vec();
    if files.is_empty() {
      return Ok(entry);
    }
    return entry.get_file_rec(files);
  }

  pub fn get_file(&self, path: &str) -> Result<&Entry, String>{
    let files = utils::resolve_path_files(path);
    return self.get_file_rec(&files)
  }

  pub async fn load(&self) -> Result<String, String> {
    let url = ROOT_URL.to_string()+self.url;
    info!("loading url: {}", url);
    utils::fetch(url).await
  }

}