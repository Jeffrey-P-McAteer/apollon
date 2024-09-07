
use std::collections::HashMap;

use crate::structs;

pub async fn read_ld_file(path: &std::path::Path) -> Vec<HashMap<String, structs::Value>> {
  let mut v: Vec<HashMap<String, structs::Value>> = vec![];

  if let Ok(file_string_content) = tokio::fs::read_to_string(path).await {
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

      // We cannot hold a ref to the headers b/c it creates a mutable borrow of `rdr`.
      // We instead use a temp mut borrow to parse, then clone the result.
      let _temp_empty_str_rec = csv::StringRecord::new();
      let csv_headers = rdr.headers().unwrap_or(&_temp_empty_str_rec).clone();
      let num_headers = csv_headers.len();

      let mut iter = rdr.records();

      while let Some(one_row) = iter.next() {
        if let Ok(row_str_rec) = one_row {
          let mut parsed_row = HashMap::<String, structs::Value>::new();

          for col_i in 0..num_headers {
            if let (Some(header_s), Some(val_s)) = (csv_headers.get(col_i), row_str_rec.get(col_i)) {
              parsed_row.insert(header_s.to_string(), structs::Value::from_str(val_s));
            }
          }

          v.push(parsed_row);
        }
      }

    }
  }

  return v;
}
