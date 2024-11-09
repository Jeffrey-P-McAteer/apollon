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

  // Compile cl_kernel source code to programs
  let kernel_compile_start = std::time::Instant::now();
  for i in 0..cl_kernels.len() {
    cl_kernels[i].load_program(&context)?;
  }
  let kernel_compile_end = std::time::Instant::now();
  eprintln!("CL Kernel Compile Time: {}", utils::duration_to_display_str(&(kernel_compile_end - kernel_compile_start)));

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
  }

  let simulation_end = std::time::Instant::now();
  eprintln!("Simulation Time: {}", utils::duration_to_display_str(&(simulation_end - simulation_start)));

  // Write to simcontrol.output_data_file_path
  utils::write_ld_file(args, &sim_data, &simcontrol.output_data_file_path).await?;

  let total_end = std::time::Instant::now();
  eprintln!("Total Time: {}", utils::duration_to_display_str(&(total_end - total_start)));

/*
  use opencl3::command_queue::{CommandQueue, CL_QUEUE_PROFILING_ENABLE};
  use opencl3::context::Context;
  use opencl3::device::{get_all_devices, Device, CL_DEVICE_TYPE_GPU};
  use opencl3::kernel::{ExecuteKernel, Kernel};
  use opencl3::memory::{Buffer, CL_MEM_READ_ONLY, CL_MEM_WRITE_ONLY};
  use opencl3::program::Program;
  use opencl3::types::{cl_event, cl_float, CL_BLOCKING, CL_NON_BLOCKING};
  use opencl3::Result;
  use std::ptr;


const PROGRAM_SOURCE: &str = r#"
kernel void saxpy_float (global float* z,
    global float const* x,
    global float const* y,
    float a)
{
    const size_t i = get_global_id(0);
    z[i] = a*x[i] + y[i];
}"#;

const KERNEL_NAME: &str = "saxpy_float";

  // Create a Context on an OpenCL device
  let context = Context::from_device(&device).expect("Context::from_device failed");

  // Create a command_queue on the Context's device
  let queue = CommandQueue::create_default(&context, CL_QUEUE_PROFILING_ENABLE)
      .expect("CommandQueue::create_default failed");

  // Build the OpenCL program source and create the kernel.
  let program = Program::create_and_build_from_source(&context, PROGRAM_SOURCE, "")
      .expect("Program::create_and_build_from_source failed");
  let kernel = Kernel::create(&program, KERNEL_NAME).expect("Kernel::create failed");

  /////////////////////////////////////////////////////////////////////
  // Compute data

  // The input data
  const ARRAY_SIZE: usize = 1000;
  let ones: [cl_float; ARRAY_SIZE] = [1.0; ARRAY_SIZE];
  let mut sums: [cl_float; ARRAY_SIZE] = [0.0; ARRAY_SIZE];
  for i in 0..ARRAY_SIZE {
      sums[i] = 1.0 + 1.0 * i as cl_float;
  }

  // Create OpenCL device buffers
  let mut x = unsafe {
      Buffer::<cl_float>::create(&context, CL_MEM_READ_ONLY, ARRAY_SIZE, ptr::null_mut())?
  };
  let mut y = unsafe {
      Buffer::<cl_float>::create(&context, CL_MEM_READ_ONLY, ARRAY_SIZE, ptr::null_mut())?
  };
  let z = unsafe {
      Buffer::<cl_float>::create(&context, CL_MEM_WRITE_ONLY, ARRAY_SIZE, ptr::null_mut())?
  };

  // Blocking write
  let _x_write_event = unsafe { queue.enqueue_write_buffer(&mut x, CL_BLOCKING, 0, &ones, &[])? };

  // Non-blocking write, wait for y_write_event
  let y_write_event =
      unsafe { queue.enqueue_write_buffer(&mut y, CL_NON_BLOCKING, 0, &sums, &[])? };

  // a value for the kernel function
  let a: cl_float = 300.0;

  // Use the ExecuteKernel builder to set the kernel buffer and
  // cl_float value arguments, before setting the one dimensional
  // global_work_size for the call to enqueue_nd_range.
  // Unwraps the Result to get the kernel execution event.
  let kernel_event = unsafe {
      ExecuteKernel::new(&kernel)
          .set_arg(&z)
          .set_arg(&x)
          .set_arg(&y)
          .set_arg(&a)
          .set_global_work_size(ARRAY_SIZE)
          .set_wait_event(&y_write_event)
          .enqueue_nd_range(&queue)?
  };

  let mut events: Vec<cl_event> = Vec::default();
  events.push(kernel_event.get());

  // Create a results array to hold the results from the OpenCL device
  // and enqueue a read command to read the device buffer into the array
  // after the kernel event completes.
  let mut results: [cl_float; ARRAY_SIZE] = [0.0; ARRAY_SIZE];
  let read_event =
      unsafe { queue.enqueue_read_buffer(&z, CL_NON_BLOCKING, 0, &mut results, &events)? };

  // Wait for the read_event to complete.
  read_event.wait()?;

  // Output the first and last results
  println!("results front: {}", results[0]);
  println!("results back: {}", results[ARRAY_SIZE - 1]);

  // Calculate the kernel duration, from the kernel_event
  let start_time = kernel_event.profiling_command_start()?;
  let end_time = kernel_event.profiling_command_end()?;
  let duration = end_time - start_time;

  use num_format::{Locale, ToFormattedString};
  println!("kernel execution duration (ns): {}", duration.to_formatted_string(&Locale::en) );

*/

  Ok(())
}

