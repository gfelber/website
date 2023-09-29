use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use serde::Serialize;

fn main() {
  let out_dir = env::var_os("OUT_DIR").unwrap();
  let dest_path = Path::new(&out_dir).join("root.rs");
  let cargo_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();
  let root_path = Path::new(&cargo_dir).join("root");

  let mut root: Entry = Entry {
    filename: Box::new("".to_string()),
    url: Box::new("".to_string()),
    size: root_path.metadata().unwrap().len(),
    modified: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
    is_dir: true,
    entries: HashMap::new(),
  };
  visit_dirs(&mut root, root_path.as_path(), "").expect("couldn't read dir");
  let root_serialized: String = ron::ser::to_string(&root).unwrap();
  let out = format!("pub const ROOT_SERIALIZED: &str = \"{}\";\n", root_serialized.replace("\"", "\\\""));
  fs::write(
    &dest_path,
    out,
  ).unwrap();
}

fn visit_dirs<'a>(root: &'a mut Entry, dir: &'a Path, url: &'a str) -> Result<&'a mut Entry, Box<dyn Error>> {
  if dir.is_dir() {
    for entry in fs::read_dir(dir)? {
      let dir_entry = entry?;
      let path = dir_entry.path();
      let entry_metadata = if dir_entry.file_type().unwrap().is_symlink() {
        dir_entry.path().metadata().unwrap()
      } else {
        path.metadata().unwrap()
      };
      let filename = dir_entry.file_name().to_string_lossy().to_string();
      let filename_box = Box::new(filename.clone());
      let fileurl = url.to_string() + &filename;
      let mut entry: Entry = Entry {
        filename: filename_box.clone(),
        url: Box::new(fileurl.clone()),
        size: entry_metadata.len(),
        modified: entry_metadata.modified().unwrap().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
        is_dir: path.is_dir(),
        entries: HashMap::new(),
      };

      if path.is_dir() {
        // It's a subdirectory, so visit it recursively
        visit_dirs(&mut entry, path.as_path(), &(fileurl + "/"))?;
      }

      root.entries.insert(filename_box, entry);
    }
  }

  Ok(root)
}

#[derive(Serialize)]
struct Entry {
  filename: Box<String>,
  url: Box<String>,
  size: u64,
  modified: u64,
  is_dir: bool,
  entries: HashMap<Box<String>, Entry>, // only applicable to Dirs
}
