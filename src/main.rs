// Guess who doesn't care right now?
#![allow(unused_variables)]
#![allow(dead_code)]

use clap::Parser;

pub mod structs;
pub mod utils;


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

  let device_ids = opencl3::device::get_all_devices(opencl3::device::CL_DEVICE_TYPE_GPU)?;
  // println!("device_ids = {:?}", device_ids);

  for device_id in device_ids {
    let d = opencl3::device::Device::new(device_id);
    println!("{:?} > {:?}", device_id, d.name() );
  }



  Ok(())
}

