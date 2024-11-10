// Guess who doesn't care right now?
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(non_camel_case_types)]
#![allow(unreachable_code)]

use std::cell::{RefCell, RefMut};
use std::rc::Rc;

use clap::Parser;
use num_format::{Locale, ToFormattedString};
use plotters::prelude::*;
use plotters::coord::types::RangedCoordf32;

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
      eprintln!("[ main_async ] {}", e);
    }
  });

  Ok(())
}




async fn main_async(args: &structs::Args) -> Result<(), Box<dyn std::error::Error>> {
  let total_start = std::time::Instant::now();

  let mut simcontrol = utils::read_simcontrol_file(&args.simcontrol_file_path).await?;
  // Overwrite any simcontrol args w/ cli-specified args
  utils::inplace_update_simcontrol_from_args(&mut simcontrol, args);
  let simcontrol = simcontrol;

  if args.verbose >= 2 {
    println!("simcontrol = {:#?}", simcontrol);
  }


  let pref_dev_id = utils::get_pref_device(&simcontrol.preferred_gpu_name.to_lowercase()).await?;
  let device = opencl3::device::Device::new(pref_dev_id);
  if let Ok(name) = device.name() {
    if args.verbose >= 1 {
      println!("Selected Compute device: {}", name);
    }
  }

  let t0_data = utils::read_ld_file(&simcontrol.input_data_file_path).await;
  let mut cl_kernels = utils::read_cl_kernel_file(&simcontrol.cl_kernels_file_path).await?.kernel;

  if args.verbose >= 2 {
    println!("t0_data = {:#?}", &t0_data);
    println!("cl_kernels = {:#?}", &cl_kernels);
  }

  let context = opencl3::context::Context::from_device(&device)?;

  let device_init_end = std::time::Instant::now();
  eprintln!("Hardware Initialization: {}", utils::duration_to_display_str(&(device_init_end - total_start)));

  // Compile cl_kernel source code to programs
  let kernel_compile_start = std::time::Instant::now();
  for i in 0..cl_kernels.len() {
    cl_kernels[i].load_program(&context)?;
  }
  let kernel_compile_end = std::time::Instant::now();
  eprintln!("CL Kernel Compile Time: {}", utils::duration_to_display_str(&(kernel_compile_end - kernel_compile_start)));

  let gif_plot_backend = BitMapBackend::gif(
    simcontrol.output_animation_file_path.clone(),
    (simcontrol.output_animation_width, simcontrol.output_animation_height),
    simcontrol.output_animation_frame_delay
  )?;
  let gif_root = gif_plot_backend.into_drawing_area();

  let simulation_start = std::time::Instant::now();

  // Each step we go in between ListedData (sim_data) and a utils::ld_data_to_kernel_data vector; eventually
  // the best approach is to keep everything in a utils::ld_data_to_kernel_data format & map indexes between kernels so they read/write the same data.
  let mut sim_data = t0_data.clone();

  for sim_step_i in 0..simcontrol.num_steps {
    // For each kernel, read in sim_data, process that data, then transform back mutating sim_data itself.
    for i in 0..cl_kernels.len() {
      if let Some(k) = &cl_kernels[i].cl_device_kernel {

        if args.verbose > 2 {
          println!("sim_step_i={} i={}", sim_step_i, i);
        }

        // Create a command_queue on the Context's device; 8 is a random guess at a good size
        let queue = opencl3::command_queue::CommandQueue::create_default_with_properties(&context, opencl3::command_queue::CL_QUEUE_PROFILING_ENABLE, 0).expect("CommandQueue::create_default failed");
        //let queue = opencl3::command_queue::CommandQueue::create_default(&context, opencl3::command_queue::CL_QUEUE_PROFILING_ENABLE).expect("CommandQueue::create_default failed");

        // Move data from Sim space to Kernel space; this queues blocking data writes to buffers, which are then placed into the kernel as arguments
        let mut events: Vec<opencl3::types::cl_event> = Vec::default();
        let kernel_args = utils::ld_data_to_kernel_data(&args, &simcontrol, &sim_data, &context, &cl_kernels[i], &k, &queue, &events)?;

        let mut kernel_arg_names: Vec<String> = vec![]; // Required to line up kernel_args[] indexes w/ data names
        if let Ok(argc) = k.num_args() {
          for arg_i in 0..argc {
            kernel_arg_names.push(
              k.get_arg_name(arg_i).unwrap_or(String::new())
            );
          }
        }

        // Allocate a runtime kernel & feed it inputs; we use RefCell here b/c otherwise inner-loop lifetimes would kill us
        let mut exec_kernel = opencl3::kernel::ExecuteKernel::new(&k);

        for arg in kernel_args.iter() {
          unsafe {
            match arg {
              structs::CL_TaggedArgument::Uint8Buffer(a)  => {let exec_kernel = exec_kernel.set_arg(a);},
              structs::CL_TaggedArgument::Uint16Buffer(a) => {let exec_kernel = exec_kernel.set_arg(a);},
              structs::CL_TaggedArgument::Uint32Buffer(a) => {let exec_kernel = exec_kernel.set_arg(a);},
              structs::CL_TaggedArgument::Uint64Buffer(a) => {let exec_kernel = exec_kernel.set_arg(a);},
              structs::CL_TaggedArgument::Int8Buffer(a)   => {let exec_kernel = exec_kernel.set_arg(a);},
              structs::CL_TaggedArgument::Int16Buffer(a)  => {let exec_kernel = exec_kernel.set_arg(a);},
              structs::CL_TaggedArgument::Int32Buffer(a)  => {let exec_kernel = exec_kernel.set_arg(a);},
              structs::CL_TaggedArgument::Int64Buffer(a)  => {let exec_kernel = exec_kernel.set_arg(a);},
              structs::CL_TaggedArgument::FloatBuffer(a)  => {let exec_kernel = exec_kernel.set_arg(a);},
              structs::CL_TaggedArgument::DoubleBuffer(a) => {let exec_kernel = exec_kernel.set_arg(a);},
              structs::CL_TaggedArgument::Uint8(a)        => {let exec_kernel = exec_kernel.set_arg(a);},
              structs::CL_TaggedArgument::Uint16(a)       => {let exec_kernel = exec_kernel.set_arg(a);},
              structs::CL_TaggedArgument::Uint32(a)       => {let exec_kernel = exec_kernel.set_arg(a);},
              structs::CL_TaggedArgument::Uint64(a)       => {let exec_kernel = exec_kernel.set_arg(a);},
              structs::CL_TaggedArgument::Int8(a)         => {let exec_kernel = exec_kernel.set_arg(a);},
              structs::CL_TaggedArgument::Int16(a)        => {let exec_kernel = exec_kernel.set_arg(a);},
              structs::CL_TaggedArgument::Int32(a)        => {let exec_kernel = exec_kernel.set_arg(a);},
              structs::CL_TaggedArgument::Int64(a)        => {let exec_kernel = exec_kernel.set_arg(a);},
              structs::CL_TaggedArgument::Float(a)        => {let exec_kernel = exec_kernel.set_arg(a);},
              structs::CL_TaggedArgument::Double(a)       => {let exec_kernel = exec_kernel.set_arg(a);},
            }
          }
        }

        { // Set the global work size is the number of entitied being simulated
          let exec_kernel = exec_kernel.set_global_work_size( sim_data.len() );
        }

        // Setup command queue
        let mut kernel_event = unsafe { exec_kernel.enqueue_nd_range(&queue)? };

        events.push(kernel_event.get());

        // Kernel is now running, we do NOT wait for processing to finish. Instead we pass
        // &events to kernel_data_update_ld_data, where those events will be passed to all read functions.
        // The CL runtime will guarantee the processing has completed before data is read back out.

        utils::kernel_data_update_ld_data(&context, &queue, &events, &kernel_args, &kernel_arg_names, &mut sim_data)?;

      }
      else {
        eprintln!("[ Fatal Error ] Kernel {} does not have a cl_device_kernel! Inspect hardware & s/w to ensure kernels compile when loaded.", cl_kernels[i].name);
        return Ok(());
      }
    }

    // Finally possibly render a frame of data to gif_plot
    if sim_step_i % simcontrol.capture_step_period == 0 {
      // Render!
      gif_root.fill(&WHITE)?;

      // For each entity, if an gis_x_attr_name and gis_y_attr_name coordinate are known and are numeric,
      // render a dot with a label from gis_name_attr
      for row_i in 0..sim_data.len() {
        if let (Some(x_val), Some(y_val)) = (sim_data[row_i].get(&simcontrol.gis_x_attr_name), sim_data[row_i].get(&simcontrol.gis_y_attr_name)) {
          if let (Ok(x_i32), Ok(y_i32)) = (x_val.to_i32(), y_val.to_i32()) {
            // Render!
            let mut label_s = sim_data[row_i].get(&simcontrol.gis_name_attr).map(|v| v.to_string()).unwrap_or_else(|| format!("{}", row_i));
            let elm = EmptyElement::at((x_i32, y_i32))
                + Circle::new((0, 0), 2, ShapeStyle::from(&BLACK).filled())
                + Text::new(
                    label_s,
                    (10, 0),
                    ("sans-serif", 15.0).into_font(),
              );
            gif_root.draw(&elm)?;
          }
        }
      }

      gif_root.present()?;
    }

  }

  let simulation_end = std::time::Instant::now();
  eprintln!("Simulation Time: {}", utils::duration_to_display_str(&(simulation_end - simulation_start)));

  // Write to simcontrol.output_data_file_path
  utils::write_ld_file(args, &sim_data, &simcontrol.output_data_file_path).await?;

  // Write to simcontrol.output_animation_file_path


  let total_end = std::time::Instant::now();
  eprintln!("Total Time: {}", utils::duration_to_display_str(&(total_end - total_start)));

  Ok(())
}

