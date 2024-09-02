
use std::collections::HashMap;

use clap::Parser;

pub mod structs;


fn main() {
  let args = structs::Args::parse();

  println!("args = {:?}", &args);

  let t0_data = read_ld_file(&args.data_file_path);
  let delta_data = read_ld_file(&args.delta_file_path);

  println!("t0_data = {:?}", &t0_data);
  println!("delta_data = {:?}", &delta_data);



}

pub fn read_ld_file(path: &std::path::Path) -> Vec<HashMap<String, structs::Value>> {
  let mut v: Vec<HashMap<String, structs::Value>> = vec![];

  if let Ok(file_string_content) = std::fs::read_to_string(path) {
    if let Ok(mut file_json_content) = serde_jsonrc::from_str(&file_string_content) {
      v.append(&mut file_json_content);
    }
    else {
      // Report any JSON errors IF path ends in .json
      let mut ext = path.extension().unwrap_or(std::ffi::OsStr::new("")).to_string_lossy().to_string();
      ext.make_ascii_lowercase();
      let has_json_ext = ext == "json";
      if has_json_ext {
        if let Err(e) = serde_jsonrc::from_str::<Vec<HashMap<String, structs::Value>>>(&file_string_content) {
          println!("{} JSON parse error: {:?}", path.display(), e);
        }
      }

      // Continue attempting parse formats

      let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true) // Allow empty colums on some csv lines
        .from_reader(file_string_content.as_bytes());

      let mut iter = rdr.records();

      while let Some(one_row) = iter.next() {
        if let Ok(row_str_rec) = one_row {
          //v.push(one_map);
        }
      }

    }
  }

  return v;
}


