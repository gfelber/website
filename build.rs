use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;

use std::path::Path;
use std::time::SystemTime;

use serde::Serialize;
use chrono::{DateTime, Utc};

fn main() {
  let out_dir = env::var_os("OUT_DIR").unwrap();
  let dest_path = Path::new(&out_dir).join("root.rs");
  let cargo_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();
  let root_path = Path::new(&cargo_dir).join("root");
  let index_path = Path::new(&cargo_dir).join("www/index.html");
  let sitemap_path = Path::new(&cargo_dir).join("www/sitemap.xml");

  let mut root: Entry = Entry {
    filename: Box::new("".to_string()),
    url: Box::new("".to_string()),
    size: root_path.metadata().unwrap().len(),
    modified: SystemTime::now()
      .duration_since(SystemTime::UNIX_EPOCH)
      .unwrap()
      .as_secs(),
    is_dir: true,
    entries: HashMap::new(),
  };
  visit_dirs(&mut root, root_path.as_path(), "").expect("couldn't read dir");
  create_dirs(&root, &index_path);
  generate_sitemap(&root, &sitemap_path).expect("couldn't generate sitemap");
  let root_serialized: String = ron::ser::to_string(&root).unwrap();
  let out = format!(
    "pub const ROOT_SERIALIZED: &str = \"{}\";\n",
    root_serialized.replace("\"", "\\\"")
  );
  fs::write(&dest_path, out).unwrap();
}

const PARENT_URL: &str = "dirs/";
const BASE_URL: &str = "https://www.gfelber.dev";

fn generate_sitemap(root: &Entry, sitemap_path: &Path) -> Result<(), Box<dyn Error>> {
  let mut urls = Vec::new();

  // Add homepage
  urls.push(format!(
    "  <url>\n    <loc>{}/</loc>\n    <lastmod>{}</lastmod>\n    <changefreq>weekly</changefreq>\n    <priority>1.0</priority>\n  </url>",
    BASE_URL,
    format_timestamp(root.modified)
  ));

  // Collect all URLs from the root structure
  collect_urls(&mut urls, root, "");

  let sitemap = format!(
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n{}\n</urlset>",
    urls.join("\n")
  );

  fs::write(sitemap_path, sitemap)?;
  Ok(())
}

fn format_timestamp(timestamp: u64) -> String {
  let datetime = DateTime::<Utc>::from_timestamp(timestamp as i64, 0)
    .unwrap_or_else(|| Utc::now());
  datetime.format("%Y-%m-%d").to_string()
}

fn collect_urls(urls: &mut Vec<String>, entry: &Entry, parent_url: &str) {
  for (_filename, child) in entry.entries.iter() {
    let url = format!("{}/{}", parent_url, child.filename);
    let clean_url = url.trim_start_matches('/');

    // Skip res and img directories, and latest.md
    if clean_url.starts_with("res/") || clean_url.starts_with("img/") ||
       clean_url.contains("/res/") || clean_url.contains("/img/") ||
       clean_url == "latest.md" {
      continue;
    }

    // Add both files and directories to sitemap
    let priority = if child.is_dir { "0.8" } else { "0.6" };
    let changefreq = if child.is_dir { "weekly" } else { "monthly" };
    
    urls.push(format!(
      "  <url>\n    <loc>{}/{}/</loc>\n    <lastmod>{}</lastmod>\n    <changefreq>{}</changefreq>\n    <priority>{}</priority>\n  </url>",
      BASE_URL,
      clean_url,
      format_timestamp(child.modified),
      changefreq,
      priority
    ));

    // If URL contains "old/", also add a fallback URL without it
    if clean_url.contains("/old/") {
      let fallback_url = clean_url.replace("/old/", "/");
      urls.push(format!(
        "  <url>\n    <loc>{}/{}/</loc>\n    <lastmod>{}</lastmod>\n    <changefreq>{}</changefreq>\n    <priority>{}</priority>\n  </url>",
        BASE_URL,
        fallback_url,
        format_timestamp(child.modified),
        changefreq,
        priority
      ));
    }

    // Recursively collect URLs from subdirectories
    if child.is_dir {
      collect_urls(urls, child, &url);
    }
  }
}

fn create_dirs(root: &Entry, index_path: &Path) {
  let index_template = fs::read_to_string(index_path).unwrap();
  create_dirs_with_template(root, &index_template);
}

fn create_dirs_with_template(root: &Entry, index_template: &str) {
  let _ = fs::create_dir_all(PARENT_URL.to_string() + &root.url);

  for (_filename, entry) in root.entries.iter() {
    create_dirs_with_template(entry, index_template);
  }

  if root.is_dir {
    // For directories, create an index with a description of the contents
    let description = generate_directory_description(root);
    write_index_html_for_entry(root, &description, index_template);
  } else {
    let description = get_file_description(&root.url);
    write_index_html_for_entry(root, &description, index_template);

    if root.url.contains("old/")  {
      let fallback_url = root.url.replace("old/", "");
      let _ = fs::create_dir_all(PARENT_URL.to_string() + &fallback_url);
      let mut fallback_entry = root.clone();
      fallback_entry.url = Box::new(fallback_url);
      write_index_html_for_entry(&fallback_entry, &description, index_template);
    }
  }
}

fn generate_directory_description(entry: &Entry) -> String {
  let mut file_list = Vec::new();

  for (_filename, child) in entry.entries.iter() {
    if !child.filename.starts_with(".") {
      file_list.push(child.filename.as_str());
    }
  }

  if !file_list.is_empty() {
    let mut desc = format!("Directory containing: {}", file_list.join(", "));
    if desc.len() > 500 {
      desc.truncate(497);
      desc.push_str("...");
    }
    desc
  } else {
    String::from("Directory")
  }
}

fn get_file_description(entry_path: &str) -> String {
  let cargo_dir = env::var_os("CARGO_MANIFEST_DIR").unwrap();
  let md_path = Path::new(&cargo_dir).join(entry_path);

  if md_path.exists() && md_path.is_file() {
    extract_description(&md_path)
  } else {
    String::new()
  }
}

fn write_index_html_for_entry(entry: &Entry, description: &str, index_template: &str) {
  // Extract filename from URL
  let filename = entry.url.trim_end_matches('/').split('/').last().unwrap_or(&entry.url);
  let escaped_filename = escape_html(filename);
  
  let mut modified_html = if !description.is_empty() {
    let escaped_description = escape_html(description);
    index_template.replace(
      "<head>",
      &format!("<head>\n    <meta name=\"description\" content=\"{}\" />", escaped_description)
    )
  } else {
    index_template.to_string()
  };
  
  // Add hidden h1 with filename and links after <body>
  let mut hidden_content = format!("<h1 style=\"display: none;\">{}</h1>", escaped_filename);
  
  // For directories, add hidden links to all entries
  if entry.is_dir {
    let mut links = Vec::new();
    for (_filename, child) in entry.entries.iter() {
      if !child.filename.starts_with(".") {
        let child_url = format!("/{}/", child.url.trim_end_matches('/'));
        let escaped_child_name = escape_html(&child.filename);
        links.push(format!("<a href=\"{}\">{}</a>", child_url, escaped_child_name));
      }
    }
    if !links.is_empty() {
      hidden_content.push_str(&format!("\n    <nav style=\"display: none;\">{}</nav>", links.join(" ")));
    }
  }
  
  modified_html = modified_html.replace(
    "<body>",
    &format!("<body>\n    {}", hidden_content)
  );

  let dest_path = PARENT_URL.to_string() + &entry.url + "/index.html";
  let _ = fs::write(dest_path, modified_html);
}

fn escape_html(text: &str) -> String {
  text
    .replace("&", "&amp;")
    .replace("\"", "&quot;")
    .replace("<", "&lt;")
    .replace(">", "&gt;")
}

fn extract_description(md_path: &Path) -> String {
  let content = match fs::read_to_string(md_path) {
    Ok(c) => c,
    Err(_) => return String::new(),
  };

  if let Some(tldr_pos) = content.to_lowercase().find("tl;dr") {
    if let Some(line_end) = content[tldr_pos..].find('\n') {
      let start = tldr_pos + line_end + 1;
      let remaining = &content[start..];

      let mut description = String::new();
      for line in remaining.lines() {
        if line.trim().starts_with('#') || line.trim().starts_with("```") {
          break;
        }
        if description.len() + line.len() > 500 {
          let space_left = 500 - description.len();
          description.push_str(&line[..space_left.min(line.len())]);
          break;
        }
        if !description.is_empty() {
          description.push(' ');
        }
        description.push_str(line.trim());
      }

      return description.trim().to_string();
    }
  }

  let text: String = content
    .lines()
    .filter(|line| !line.trim().starts_with('#'))
    .map(|line| line.trim())
    .filter(|line| !line.is_empty())
    .collect::<Vec<&str>>()
    .join(" ");

  text.chars().take(500).collect()
}

fn visit_dirs<'a>(
  root: &'a mut Entry,
  dir: &'a Path,
  url: &'a str,
) -> Result<&'a mut Entry, Box<dyn Error>> {
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
        modified: entry_metadata
          .modified()
          .unwrap()
          .duration_since(SystemTime::UNIX_EPOCH)
          .unwrap()
          .as_secs(),
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

#[derive(Serialize, Clone)]
struct Entry {
  filename: Box<String>,
  url: Box<String>,
  size: u64,
  modified: u64,
  is_dir: bool,
  entries: HashMap<Box<String>, Entry>, // only applicable to Dirs
}
