// Guess who doesn't care right now?
#![allow(unused_variables)]
#![allow(dead_code)]

use clap::Parser;

pub mod structs;
pub mod utils;

// From hello world code
use emu_glsl::*;
use emu_core::prelude::*;
use zerocopy::*;

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



#[repr(C)]
#[derive(zerocopy::FromZeroes, zerocopy::AsBytes, zerocopy::FromBytes, Copy, Clone, Default, Debug)]
pub struct Rectangle {
    x: u32,
    y: u32,
    w: i32,
    h: i32,
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

  // first, we move a bunch of rectangles to the GPU
  let mut x: DeviceBox<[Rectangle]> = vec![Default::default(); 128].as_device_boxed()?;

  // then we compile some GLSL code using the GlslCompile compiler and
  // the GlobalCache for caching compiler artifacts
  //let c = compile::<String, GlslCompile, _, GlobalCache>(
  let c = compile::<String, _, _, GlobalCache>(
      GlslBuilder::new()
          .set_entry_point_name("main")
          .add_param_mut()
          .set_code_with_glsl(
          r#"
#version 450
layout(local_size_x = 1) in; // our thread block size is 1, that is we only have 1 thread per block

struct Rectangle {
    uint x;
    uint y;
    int w;
    int h;
};

// make sure to use only a single set and keep all your n parameters in n storage buffers in bindings 0 to n-1
// you shouldn't use push constants or anything OTHER than storage buffers for passing stuff into the kernel
// just use buffers with one buffer per binding
layout(set = 0, binding = 0) buffer Rectangles {
    Rectangle[] rectangles;
}; // this is used as both input and output for convenience

Rectangle flip(Rectangle r) {
    r.x = r.x + r.w;
    r.y = r.y + r.h;
    r.w *= -1;
    r.h *= -1;
    return r;
}

// there should be only one entry point and it should be named "main"
// ultimately, Emu has to kind of restrict how you use GLSL because it is compute focused
void main() {
    uint index = gl_GlobalInvocationID.x; // this gives us the index in the x dimension of the thread space
    rectangles[index] = flip(rectangles[index]);
}
            "#,
      )
  )?.finish()?;

  // we spawn 128 threads (really 128 thread blocks)
  unsafe {
      spawn(128).launch(call!(c, &mut x));
  }

  // this is the Future we need to block on to get stuff to happen
  // everything else is non-blocking in the API (except stuff like compilation)
  println!("{:?}", x.get().await?);



  Ok(())
}

