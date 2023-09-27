use serde::Deserialize;
include!(concat!(env!("OUT_DIR"), "/root.rs"));

#[derive(Deserialize)]
pub struct Entry {
  pub filename: &'static str,
  pub size: u64,
  pub modified: u64,
  pub is_dir: bool,
  pub children: Vec<Entry>, // only applicable to Dirs
}

impl Entry{
  pub fn new() -> Entry{
    return ron::from_str(ROOT).unwrap();
  }

}