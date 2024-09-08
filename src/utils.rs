
use std::collections::HashMap;

use crate::structs;

// ld == "Listed Data", it's shape must be Vec<Map<string, object>>
pub async fn read_ld_file(path: &std::path::Path) -> Vec<HashMap<String, structs::Value>> {
  let mut v: Vec<HashMap<String, structs::Value>> = vec![];

  if let Ok(file_string_content) = tokio::fs::read_to_string(path).await {
    if let Ok(mut file_toml_content) = toml::from_str(&file_string_content) {
      v.append(&mut file_toml_content);
    }
    else if let Ok(mut file_json_content) = serde_jsonrc::from_str(&file_string_content) {
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

pub async fn read_cl_kernel_file(path: &std::path::Path) -> structs::CL_Kernels {
  let mut v: structs::CL_Kernels = structs::CL_Kernels::default();

  if let Ok(file_string_content) = tokio::fs::read_to_string(path).await {
    if let Ok(mut file_toml_content) = toml::from_str::<structs::CL_Kernels>(&file_string_content) {
      v.kernel.append(&mut file_toml_content.kernel);
    }
    else if let Ok(mut file_json_content) = serde_jsonrc::from_str::<structs::CL_Kernels>(&file_string_content) {
      v.kernel.append(&mut file_json_content.kernel);
    }
    // Not supporting other file formats, that's dumb.
  }

  return v;
}

pub async fn get_pref_device(args: &structs::Args) -> Result<opencl3::types::cl_device_id, Box<dyn std::error::Error>> {

  let lower_pref_name = args.preferred_gpu_name.to_lowercase();
  let mut gpu_device_ids = opencl3::device::get_all_devices(opencl3::device::CL_DEVICE_TYPE_GPU)?;
  gpu_device_ids.append(
    &mut opencl3::device::get_all_devices(opencl3::device::CL_DEVICE_TYPE_CPU)?
  );
  // ^^ also opencl3::device::CL_DEVICE_TYPE_ALL

  let gpu_device_ids = gpu_device_ids;

  if lower_pref_name.len() > 0 {
    // List if requested
    if lower_pref_name == "list" {
      for device_id in &gpu_device_ids {
        let d = opencl3::device::Device::new(*device_id);
        if let Ok(name) = d.name() {
          println!("{: <32} max_compute_units={: <3} max_clock_frequency={: <5} max_work_group_size={: <5}",
            name,
            d.max_compute_units().unwrap_or(0),
            d.max_clock_frequency().unwrap_or(0),
            d.max_work_group_size().unwrap_or(0)
          );
        }
      }
      return Err(Box::from("Listing GPUs complete"));
    }
    // Search & return first match
    for device_id in &gpu_device_ids {
      let d = opencl3::device::Device::new(*device_id);
      if let Ok(name) = d.name() {
        let name = name.to_lowercase();
        if name.contains(&lower_pref_name) {
          return Ok(*device_id);
        }
      }
    }
  }

  // No preferred GPU device name, return the greatest of .max_compute_units() * .max_work_group_size() from all GPUs
  let mut largest_compute_id = *( gpu_device_ids.first().clone().ok_or("No compute devices available!")? );
  let mut largest_compute_score: usize = 0;
  for device_id in &gpu_device_ids {
    let d = opencl3::device::Device::new(*device_id);
    let score = d.max_compute_units().unwrap_or(0) as usize * d.max_work_group_size().unwrap_or(0);
    if score > largest_compute_score {
      largest_compute_id = *device_id;
      largest_compute_score = score as usize;
    }
  }

  return Ok(largest_compute_id);

}
