// Guess who doesn't care right now?
#![allow(unused_variables)]
#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_mut)]
#![allow(non_camel_case_types)]
#![allow(unreachable_code)]

use std::cell::{RefCell, RefMut};
use std::rc::Rc;
use std::borrow::Borrow;

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
      std::process::exit(1);
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

  let mut gif_point_history: Vec<(i32, i32)> = vec![];

  let simulation_start = std::time::Instant::now();

  // Each step we go in between ListedData (sim_data) and a utils::ld_data_to_kernel_data vector; eventually
  // the best approach is to keep everything in a utils::ld_data_to_kernel_data format & map indexes between kernels so they read/write the same data.
  let mut sim_data = t0_data.clone();

  // For performance reasons we pre-allocate all entity colors here and re-use
  // when plotting data. This means there will be NO capability to change an entity color in the middle of
  // a sim; and if there were I'd want to provide the API as an "index into known colors" anyhow.
  let mut sim_data_colors: Vec<plotters::style::RGBColor> = vec![];
  for row in sim_data.iter() {
    if let Some(str_val) = row.get(&simcontrol.gis_color_attr) {
      match csscolorparser::parse(str_val.to_string().as_str()) {
        Ok(css_color_obj) => {
          let components = css_color_obj.to_rgba8();
          sim_data_colors.push( plotters::style::RGBColor(components[0], components[1], components[2]) );
        }
        Err(e) => {
          if args.verbose > 0 {
            eprintln!("{:?}", e);
          }
         sim_data_colors.push(plotters::style::colors::BLACK);
        }
      }
    }
    else {
       sim_data_colors.push(plotters::style::colors::BLACK);
    }
  }

  let mut total_kernel_execs_duration = std::time::Duration::from_millis(0);
  let mut total_convert_overhead_duration = std::time::Duration::from_millis(0);
  let mut total_gis_paint_duration = std::time::Duration::from_millis(0);

  // Allocate long-term CL data
  let queue = opencl3::command_queue::CommandQueue::create_default_with_properties(&context, opencl3::command_queue::CL_QUEUE_PROFILING_ENABLE, 0).expect("CommandQueue::create_default failed");

  // Both vectors must be kept in-sync; we keep sim_events_cl so we can rapidly pass a pointer to always-valid CL event structures
  let mut sim_events: Vec<opencl3::event::Event> = vec![];
  let mut sim_events_cl: Vec<opencl3::types::cl_event> = vec![];

  // For each kernel, convert LD data to Kernel data;
  // For each new (Name,Type) pair add to a all_kernel vector of tagged CL buffers.
  // We then store argument indexes into the all_kernel vector for individual kernels,
  // allowing re-use of the same buffers across the entire simulation.
  let mut all_kernel_args: Vec<structs::CL_NamedTaggedArgument> = vec![];
  let mut all_kernel_arg_indicies: Vec<Vec<usize>> = vec![];
  for i in 0..cl_kernels.len() {
    if let Some(k) = &cl_kernels[i].cl_device_kernel {

      let ld_to_kernel_start = std::time::Instant::now();
      let kernel_args = utils::ld_data_to_kernel_data_named(&args, &simcontrol, &sim_data, &context, &cl_kernels[i], &k, &queue, &sim_events_cl)?;
      let ld_to_kernel_end = std::time::Instant::now();
      total_convert_overhead_duration += ld_to_kernel_end - ld_to_kernel_start;

      let mut this_kernel_ak_indicies: Vec<usize> = vec![];

      for kai in 0..kernel_args.len() {
        let mut all_kernel_args_existing_idx: Option<usize> = None;
        for akai in 0..all_kernel_args.len() {
          if kernel_args[kai].name == all_kernel_args[akai].name && std::mem::discriminant::<structs::CL_TaggedArgument>(kernel_args[kai].tagged_argument.borrow()) == std::mem::discriminant::<structs::CL_TaggedArgument>(all_kernel_args[akai].tagged_argument.borrow()) {
            // Name & Type matches, store index directly
            all_kernel_args_existing_idx = Some(akai);
            break;
          }
        }

        match all_kernel_args_existing_idx {
          Some(akai_idx) => {
            this_kernel_ak_indicies.push(akai_idx);
          }
          None => {
            // New name,type must be added to all_kernel_args.
            // Calling .clone() will make the interior .tagged_argument read-only until kernel_args is dropped at the end of this cl_kernels[i] loop iteration.
            this_kernel_ak_indicies.push(all_kernel_args.len());
            all_kernel_args.push(
              kernel_args[kai].clone()
            );
          }
        }

      }

      all_kernel_arg_indicies.push(this_kernel_ak_indicies);

    }
  }

  // Inspect & Panic if any of the interior .tagged_argument Arcs are not mutable; we require these to be mutable downstairs.
  for akai in 0..all_kernel_args.len() {
    if std::sync::Arc::<structs::CL_TaggedArgument>::get_mut(&mut all_kernel_args[akai].tagged_argument).is_none() {
      eprintln!("Logic error! all_kernel_args[{}].tagged_argument was supposed to be mutable, but is not!", akai);
      panic!("Logic error!");
    }
  }
  if args.verbose > 0 {
    eprintln!("all_kernel_arg_indicies = {:?}", all_kernel_arg_indicies);
  }

  // Finally, we must create & inject "Conversion Kernels" into the stream where we have
  // Variable A of type A followed by Variable A of type B in all_kernel_args.
  // ^^ TODO

  // Allocate shared GUI render mem
  let sans_serif_font = ("sans-serif", 15.0).into_font();
  let monospace_font = ("monospace", 14.0).into_font();

  for sim_step_i in 0..simcontrol.num_steps {
    // For each kernel, read in sim_data, process that data, then transform back mutating sim_data itself.
    for i in 0..cl_kernels.len() {
      if let Some(k) = &cl_kernels[i].cl_device_kernel {

        let kernel_exec_start = std::time::Instant::now();

        // Allocate a runtime kernel & feed it inputs; we use RefCell here b/c otherwise inner-loop lifetimes would kill us
        let mut exec_kernel = opencl3::kernel::ExecuteKernel::new(&k);

        for aka_idx in all_kernel_arg_indicies[i].iter() {
          let arg = &all_kernel_args[*aka_idx].clone();
          unsafe {
            match arg.tagged_argument.borrow() {
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

        // Safety: both vectors increase at same time
        sim_events_cl.push(kernel_event.get());
        sim_events.push(kernel_event);

        // Kernel is now running, we do NOT wait for processing to finish. Instead we pass
        // &events to kernel_data_update_ld_data, where those events will be passed to all read functions.
        // The CL runtime will guarantee the processing has completed before data is read back out.

        // Update: nvm we do wait b/c timing!
        //kernel_event.wait()?;

        let kernel_exec_end = std::time::Instant::now();
        total_kernel_execs_duration += kernel_exec_end - kernel_exec_start;

/*        let kernel_to_ld_start = std::time::Instant::now();
        utils::kernel_data_update_ld_data(&args, &context, &queue, &sim_events_cl, &kernel_args, &kernel_arg_names, &mut sim_data)?;
        let kernel_to_ld_end = std::time::Instant::now();
        total_convert_overhead_duration += kernel_to_ld_end - kernel_to_ld_start;
*/

      }
      else {
        eprintln!("[ Fatal Error ] Kernel {} does not have a cl_device_kernel! Inspect hardware & s/w to ensure kernels compile when loaded.", cl_kernels[i].name);
        return Ok(());
      }
    }

    // Every N or so steps trim the events vector on the assumption some have completed
    if sim_step_i % 10 == 0 {
      utils::trim_completed_events(&args, &mut sim_events, &mut sim_events_cl)?;
    }

    // Finally possibly render a frame of data to gif_plot
    if sim_step_i % simcontrol.capture_step_period == 0 {

      let kernel_to_ld_start = std::time::Instant::now();
      utils::kernel_data_update_ld_data_named(&args, &context, &queue, &sim_events_cl, &all_kernel_args, &mut sim_data)?;
      let kernel_to_ld_end = std::time::Instant::now();
      total_convert_overhead_duration += kernel_to_ld_end - kernel_to_ld_start;

      let render_start = std::time::Instant::now();
      // Render!
      gif_root.fill(&WHITE)?;

      // Render entity histories as small dots
      for (historic_x, historic_y) in gif_point_history.iter() {
        let elm = EmptyElement::at((*historic_x, *historic_y)) + Circle::new((0, 0), 1, ShapeStyle::from(&RGBColor(110, 110, 110)).filled());
        gif_root.draw(&elm)?;
      }

      // For each entity, if an gis_x_attr_name and gis_y_attr_name coordinate are known and are numeric,
      // render a dot with a label from gis_name_attr
      for row_i in 0..sim_data.len() {
        if let (Some(x_val), Some(y_val)) = (sim_data[row_i].get(&simcontrol.gis_x_attr_name), sim_data[row_i].get(&simcontrol.gis_y_attr_name)) {
          if let (Ok(x_i32), Ok(y_i32)) = (x_val.to_i32(), y_val.to_i32()) {
            // Render!
            let mut label_s = sim_data[row_i].get(&simcontrol.gis_name_attr).map(|v| v.to_string()).unwrap_or_else(|| format!("{}", row_i));
            let elm = EmptyElement::at((x_i32, y_i32))
                + Circle::new((0, 0), 2, ShapeStyle::from(&sim_data_colors[row_i]).filled())
                + Text::new(
                    label_s,
                    (10, 0),
                    sans_serif_font.clone(),
              );
            gif_root.draw(&elm)?;

            gif_point_history.push( (x_i32, y_i32) );
          }
        }
      }

      // Draw sim step in lower-left corner
      let sim_step_txt = format!("{:_>9}", sim_step_i);
      let elm = Text::new(
            sim_step_txt,
            (
              (simcontrol.output_animation_width - 72) as i32,
              (simcontrol.output_animation_height - 16) as i32
            ),
            monospace_font.clone() );
      gif_root.draw(&elm)?;


      gif_root.present()?;

      let render_end = std::time::Instant::now();
      total_gis_paint_duration += render_end- render_start;
    }

  }


  if sim_events.len() > 0 {
    loop {
      if args.verbose > 0 {
        eprintln!("Waiting for {} events to complete...", sim_events.len());
      }

      for wait_i in 0..8 {
        tokio::time::sleep(tokio::time::Duration::from_millis(250)).await;

        utils::trim_completed_events(&args, &mut sim_events, &mut sim_events_cl)?;

        if sim_events.len() < 1 {
          break;
        }
      }
      if sim_events.len() < 1 {
        eprintln!("All sim events complete!");
        break;
      }
    }
  }




  let simulation_end = std::time::Instant::now();
  eprintln!("Simulation Time: {}", utils::duration_to_display_str(&(simulation_end - simulation_start)));

  eprintln!("Simulation Time Kernel Exec: {}", utils::duration_to_display_str(&total_kernel_execs_duration));
  eprintln!("Simulation Time Convert Overhead: {}", utils::duration_to_display_str(&total_convert_overhead_duration));
  eprintln!("Simulation Time Paint: {}", utils::duration_to_display_str(&total_gis_paint_duration));

  // Write to simcontrol.output_data_file_path
  utils::write_ld_file(args, &sim_data, &simcontrol.output_data_file_path).await?;

  // Write to simcontrol.output_animation_file_path


  let total_end = std::time::Instant::now();
  eprintln!("Total Time: {}", utils::duration_to_display_str(&(total_end - total_start)));

  Ok(())
}

