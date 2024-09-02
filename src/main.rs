// Guess who doesn't care right now?
#![allow(unused_variables)]
#![allow(dead_code)]

use std::collections::HashMap;

use clap::Parser;

pub mod structs;


fn main() -> Result<(), Box<dyn std::error::Error>>  {
  let args = structs::Args::parse();

  let rt  = tokio::runtime::Builder::new_multi_thread()
    .worker_threads(4)
    .thread_stack_size(8 * 1024 * 1024)
    .enable_time()
    .enable_io()
    .build()?;

  rt.block_on(async {
    if let Err(e) = main_async(&args).await {
      eprintln!("[ main_async ] {:?}", e);
    }
  });

  Ok(())
}

async fn main_async(args: &structs::Args) -> Result<(), Box<dyn std::error::Error>> {

  let all_devices = emu_core::device::Device::all().await;
  if &(args.preferred_gpu_name) == "LIST" || &(args.preferred_gpu_name) == "list" {
    println!("We have {} GPU devices: ", all_devices.len());
    for i in 0..all_devices.len() {
      if let Some(device_info) = &all_devices[i].info {
        let dt_str = format!("{:?}", device_info.device_type());
        println!("{: >3}: {: <14} {: <38} vendor=0x{:x} device=0x{:x}", i, dt_str, device_info.name(), device_info.vendor_id(), device_info.device_id());
      }
      else {
        println!("{: >3}: NO INFO", i);
      }
    }
    return Ok(());
  }


  if all_devices.len() < 1 {
    eprintln!("Fatal Error: NO GPU DEVICES!");
    return Ok(())
  }

  // Get preferred GPU device by name
  let mut gpu_device_i: usize = 0;
  if args.preferred_gpu_name.len() > 0 {
    let lower_pref_name = args.preferred_gpu_name.to_lowercase();
    for i in 0..all_devices.len() {
      if let Some(device_info) = &all_devices[i].info {
        let d_name = device_info.name().to_lowercase();
        if d_name.contains(&lower_pref_name) {
          gpu_device_i = i;
          break;
        }
      }
    }
  }
  else {
    // Grab first discrete GPU
    for i in 0..all_devices.len() {
      if let Some(device_info) = &all_devices[i].info {
        if device_info.device_type() == emu_core::device::DeviceType::DiscreteGpu {
          gpu_device_i = i;
          break;
        }
      }
    }
  }

  let mut gpu_device = &all_devices[gpu_device_i];

  println!("selected gpu_device = {:?}", gpu_device.info );


  let t0_data = read_ld_file(&args.data_file_path).await;
  let delta_data = read_ld_file(&args.delta_file_path).await;

  println!("t0_data = {:?}", &t0_data);
  println!("delta_data = {:?}", &delta_data);


  Ok(())
}


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


