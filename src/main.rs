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

  emu_core::pool::assert_device_pool_initialized().await;

  let all_devices = emu_core::device::Device::all().await;
  if &(args.preferred_gpu_name) == "LIST" || &(args.preferred_gpu_name) == "list" {
    utils::list_all_gpus(&all_devices).await;
    return Ok(());
  }

  if all_devices.len() < 1 {
    eprintln!("Fatal Error: NO GPU DEVICES!");
    return Ok(())
  }

  let gpu_device_i = utils::find_pref_gpu_i(args, &all_devices).await;
  let mut gpu_device = &all_devices[gpu_device_i];

  if let Some(gpu_info) = &gpu_device.info {
    println!("Selected GPU = {:?}", gpu_info );
  }

  let t0_data = utils::read_ld_file(&args.data_file_path).await;
  let delta_data = utils::read_ld_file(&args.delta_file_path).await;

  println!("t0_data = {:?}", &t0_data);
  println!("delta_data = {:?}", &delta_data);




  Ok(())
}

