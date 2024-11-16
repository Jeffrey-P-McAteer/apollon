
use std::collections::HashMap;
use std::borrow::Borrow;

use crate::structs;

/// General-purpose alias used to refer to loosly-typed lists of dictionaries, such as t0 data and intermediate sim steps.
pub type ListedData = Vec<HashMap<String, structs::Value>>;

// ld == "Listed Data", it's shape must be Vec<Map<string, object>>
pub async fn read_ld_file(path: &std::path::Path) -> ListedData {
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

pub async fn read_cl_kernel_file(path: &std::path::Path) -> Result<structs::CL_Kernels, Box<dyn std::error::Error>> {
  let mut v: structs::CL_Kernels = structs::CL_Kernels::default();

  if let Ok(file_string_content) = tokio::fs::read_to_string(path).await {
    if let Ok(mut file_toml_content) = toml::from_str::<structs::CL_Kernels>(&file_string_content) {
      v.kernel.append(&mut file_toml_content.kernel);
    }
    else if let Ok(mut file_json_content) = serde_jsonrc::from_str::<structs::CL_Kernels>(&file_string_content) {
      v.kernel.append(&mut file_json_content.kernel);
    }
    else {
      let mut sub_err_strs = String::new();
      if let Err(toml_e) = toml::from_str::<structs::CL_Kernels>(&file_string_content) {
        sub_err_strs = format!("{}\n{}", sub_err_strs, toml_e);
      }
      if let Err(json_e) = serde_jsonrc::from_str::<structs::CL_Kernels>(&file_string_content)  {
        sub_err_strs = format!("{}\n{}", sub_err_strs, json_e);
      }

      return Err(Box::from( format!("Error, kernel file cannot be read b/c it is not TOML or JSON data in the expected format: {}{}", path.display(), &sub_err_strs ) ));
    }
  }

  return Ok(v);
}


pub async fn read_simcontrol_file(path: &std::path::Path) -> Result<structs::SimControl, Box<dyn std::error::Error>> {

  if let Ok(file_string_content) = tokio::fs::read_to_string(path).await {

    if file_string_content.len() < 1 {
      // Empty files do not error; they just return the default values.
      return Ok(structs::SimControl::default());
    }

    // First parse the format w/ keys under [simulation]
    if let Ok(mut file_toml_content) = toml::from_str::<structs::SimControl_file>(&file_string_content) {
      file_toml_content.simulation.data_constants.extend(file_toml_content.data_constants);
      return Ok(file_toml_content.simulation);
    }
    else if let Ok(mut file_json_content) = serde_jsonrc::from_str::<structs::SimControl_file>(&file_string_content) {
      file_json_content.simulation.data_constants.extend(file_json_content.data_constants);
      return Ok(file_json_content.simulation);
    }

    // Then parse the bare format
    if let Ok(file_toml_content) = toml::from_str::<structs::SimControl>(&file_string_content) {
      return Ok(file_toml_content);
    }
    else if let Ok(file_json_content) = serde_jsonrc::from_str::<structs::SimControl>(&file_string_content) {
      return Ok(file_json_content);
    }
  }

  return Err(Box::from( format!("Error, simcontrol file cannot be read b/c it is not TOML or JSON data in the expected format: {}", path.display() ) ));
}

pub async fn write_ld_file(args: &structs::Args, ld: &ListedData, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>>  {
  // Format is selected based off file path extension
  let ext = path.extension()
    .and_then(std::ffi::OsStr::to_str)
    .unwrap_or("")
    .to_lowercase();
  match ext.as_str() {
    "json" => {
      write_ld_file_json(ld, path).await?;
    }
    "toml" => {
      write_ld_file_toml(ld, path).await?;
    }
    "csv" => {
      write_ld_file_csv(ld, path).await?;
    }
    unk_ext => {
      if args.verbose > 0 {
        eprintln!("[ Warning ] Unknown output file extension: .{}; Supported extensions are .json, .toml, .csv; Outputting as JSON", unk_ext);
      }
      write_ld_file_json(ld, path).await?;
    }
  }
  Ok(())
}

pub async fn write_ld_file_json(ld: &ListedData, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
  let json_str = serde_jsonrc::to_string(ld)?;
  std::fs::write(path, json_str+"\n")?;
  Ok(())
}
pub async fn write_ld_file_toml(ld: &ListedData, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
  let toml_str = toml::to_string(ld)?;
  std::fs::write(path, toml_str+"\n")?;
  Ok(())
}
pub async fn write_ld_file_csv(ld: &ListedData, path: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
  use std::fs::OpenOptions;
  let fd = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(path)?;
  let mut wtr = csv::Writer::from_writer(fd);

  let mut ld_keys = std::collections::HashSet::<String>::new();
  for record in ld {
    for (key, value) in record.into_iter() {
      if !ld_keys.contains(key) {
        ld_keys.insert(key.to_string());
      }
    }
  }
  // Order the keys alphabetically
  let mut ld_keys: Vec<String> = ld_keys.into_iter().collect();
  ld_keys.sort();
  let ld_keys = ld_keys;

  wtr.write_record(ld_keys.clone())?;

  for record in ld {
    let mut svalue_row: Vec<String> = vec![];
    for key in ld_keys.iter() {
      if let Some(val) = record.get(key) {
        match val {
          structs::Value::Integer(i) => svalue_row.push(format!("{}", i)),
          structs::Value::Double(d)  => svalue_row.push(format!("{}", d)),
          structs::Value::String(s)  => svalue_row.push(format!("{}", s)),
        }
      }
      else {
        svalue_row.push("".to_string());
      }
    }
    wtr.write_record(svalue_row)?;
  }


  wtr.flush()?;

  Ok(())
}


pub async fn get_pref_device(lower_pref_name: &str) -> Result<opencl3::types::cl_device_id, Box<dyn std::error::Error>> {

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
        match d.name() {
          Ok(name) => {
            println!("{: <32} max_compute_units={: <3} max_clock_frequency={: <5} max_work_group_size={: <5}",
              name,
              d.max_compute_units().unwrap_or(0),
              d.max_clock_frequency().unwrap_or(0),
              d.max_work_group_size().unwrap_or(0)
            );
          }
          Err(e) => {
            eprintln!("{:?}", e);
          }
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

pub fn inplace_update_simcontrol_from_args(simcontrol: &mut structs::SimControl, cli_args: &structs::Args) {
  if let Some(preferred_gpu_name) = &cli_args.preferred_gpu_name {
    println!("Overriding simcontrol preferred_gpu_name={} with cli arg value ={}", simcontrol.preferred_gpu_name, preferred_gpu_name);
    simcontrol.preferred_gpu_name = preferred_gpu_name.to_string();
  }

  if let Some(input_data_file_path) = &cli_args.input_data_file_path {
    println!("Overriding simcontrol input_data_file_path={} with cli arg value ={}", simcontrol.input_data_file_path.display(), input_data_file_path.display());
    simcontrol.input_data_file_path = input_data_file_path.clone();
  }

  if let Some(output_data_file_path) = &cli_args.output_data_file_path {
    println!("Overriding simcontrol output_data_file_path={} with cli arg value ={}", simcontrol.output_data_file_path.display(), output_data_file_path.display());
    simcontrol.output_data_file_path = output_data_file_path.clone();
  }

  if let Some(num_steps) = &cli_args.num_steps {
    println!("Overriding simcontrol num_steps={} with cli arg value ={}", simcontrol.num_steps, num_steps);
    simcontrol.num_steps = num_steps.clone();
  }

  if let Some(capture_step_period) = &cli_args.capture_step_period {
    println!("Overriding simcontrol capture_step_period={} with cli arg value ={}", simcontrol.capture_step_period, capture_step_period);
    simcontrol.capture_step_period = capture_step_period.clone();
  }

  if let Some(cl_kernels_file_path) = &cli_args.cl_kernels_file_path {
    println!("Overriding simcontrol cl_kernels_file_path={} with cli arg value ={}", simcontrol.cl_kernels_file_path.display(), cl_kernels_file_path.display());
    simcontrol.cl_kernels_file_path = cl_kernels_file_path.clone();
  }

  if let Some(output_animation_file_path) = &cli_args.output_animation_file_path {
    println!("Overriding simcontrol output_animation_file_path={} with cli arg value ={}", simcontrol.output_animation_file_path.display(), output_animation_file_path.display());
    simcontrol.output_animation_file_path = output_animation_file_path.clone();
  }

  if let Some(output_animation_width) = &cli_args.output_animation_width {
    println!("Overriding simcontrol output_animation_width={} with cli arg value ={}", simcontrol.output_animation_width, output_animation_width);
    simcontrol.output_animation_width = output_animation_width.clone();
  }

  if let Some(output_animation_height) = &cli_args.output_animation_height {
    println!("Overriding simcontrol output_animation_height={} with cli arg value ={}", simcontrol.output_animation_height, output_animation_height);
    simcontrol.output_animation_height = output_animation_height.clone();
  }

  if let Some(output_animation_frame_delay) = &cli_args.output_animation_frame_delay {
    println!("Overriding simcontrol output_animation_frame_delay={} with cli arg value ={}", simcontrol.output_animation_frame_delay, output_animation_frame_delay);
    simcontrol.output_animation_frame_delay = output_animation_frame_delay.clone();
  }

  if let Some(gis_x_attr_name) = &cli_args.gis_x_attr_name {
    println!("Overriding simcontrol gis_x_attr_name={} with cli arg value ={}", simcontrol.gis_x_attr_name, gis_x_attr_name);
    simcontrol.gis_x_attr_name = gis_x_attr_name.to_string();
  }

  if let Some(gis_y_attr_name) = &cli_args.gis_y_attr_name {
    println!("Overriding simcontrol gis_y_attr_name={} with cli arg value ={}", simcontrol.gis_y_attr_name, gis_y_attr_name);
    simcontrol.gis_y_attr_name = gis_y_attr_name.to_string();
  }

  if let Some(gis_name_attr) = &cli_args.gis_name_attr {
    println!("Overriding simcontrol gis_name_attr={} with cli arg value ={}", simcontrol.gis_name_attr, gis_name_attr);
    simcontrol.gis_name_attr = gis_name_attr.to_string();
  }

  if let Some(gis_color_attr) = &cli_args.gis_color_attr {
    println!("Overriding simcontrol gis_color_attr={} with cli arg value ={}", simcontrol.gis_color_attr, gis_color_attr);
    simcontrol.gis_color_attr = gis_color_attr.to_string();
  }

}


pub fn ld_data_to_kernel_data(
    args: &structs::Args,
    sc: &structs::SimControl,
    ld_data: &ListedData,
    context: &opencl3::context::Context,
    cl_kernel: &structs::CL_Kernel,
    k: &opencl3::kernel::Kernel,
    queue: &opencl3::command_queue::CommandQueue,
    events: &Vec<opencl3::types::cl_event>
  ) -> Result<Vec<structs::CL_TaggedArgument>, Box<dyn std::error::Error>>
{
  let mut kernel_data = vec![];

  let work_size = ld_data.len();
  if let Ok(argc) = k.num_args() {
    for arg_i in 0..argc {
      /*
      println!("args[{}] = {:?}, {:?}, {:?}, {:?}, {:?}", arg_i,
        k.get_arg_address_qualifier(arg_i), k.get_arg_access_qualifier(arg_i), k.get_arg_type_qualifier(arg_i),
        k.get_arg_type_name(arg_i), k.get_arg_name(arg_i)
      );
      */
      let is_pointer = k.get_arg_address_qualifier(arg_i)? == 4507;
      let is_constant = k.get_arg_type_qualifier(arg_i)? == 1;
      let type_name = k.get_arg_type_name(arg_i)?;
      let type_name = type_name.trim_end_matches('*'); // Types like 'int*' end with a star, which we do not use b/c we have is_pointer.
      let variable_name = k.get_arg_name(arg_i)?; //.unwrap_or(String::new());
      let variable_name_lowercase = variable_name.to_lowercase();
      let variable_name_uppercase = variable_name.to_uppercase();

      if is_pointer {

        // Lookup data in ld_data w/ fuzzy string matching from all records;
        // We must allocate a [T] because of the signature required by enqueue_write_buffer.
        // Because our goal is to hold massive quantities of data, we limit the buffer to some moderate stack-sized value and loop over it w/ blocking CL writes.

        let mut ld_values: Vec<structs::Value> = vec![];

        for record in ld_data.iter() {
          if let Some(val) = record.get(&variable_name) {
            ld_values.push(val.clone());
          }
          else if let Some(val) = record.get(&variable_name_lowercase) {
            ld_values.push(val.clone());
          }
          else if let Some(val) = record.get(&variable_name_uppercase) {
            ld_values.push(val.clone());
          }
          else {
            if args.verbose > 1 {
              println!("[ Warning ] Missing value for simulation data column {}, 0.0 will be used for this record.", variable_name);
            }
            ld_values.push(structs::Value::Integer(0)); // Default value regardless of type is 0, b/c we allow ld_values to contain different types & unify later
          }
        }

        let buffer_rw = if is_constant { structs::RWColumn::Read(String::new()) } else { structs::RWColumn::Write(String::new()) };

        // Now we match on the CL target type & call into the generic write_values_to_cl_buffer helper routine.
        match type_name {
          "uchar" => {
            kernel_data.push(
              structs::CL_TaggedArgument::Uint8Buffer(
                write_values_to_cl_buffer::<opencl3::types::cl_uchar>(
                context, queue, &ld_values, buffer_rw,
                |int_val| int_val as opencl3::types::cl_uchar,
                |double_val| double_val as opencl3::types::cl_uchar,
              )?)
            );
          }
          "ushort" => {
            kernel_data.push(
              structs::CL_TaggedArgument::Uint16Buffer(
                write_values_to_cl_buffer::<opencl3::types::cl_ushort>(
                context, queue, &ld_values, buffer_rw,
                |int_val| int_val as opencl3::types::cl_ushort,
                |double_val| double_val as opencl3::types::cl_ushort,
              )?)
            );
          }
          "uint" => {
            kernel_data.push(
              structs::CL_TaggedArgument::Uint32Buffer(
                write_values_to_cl_buffer::<opencl3::types::cl_uint>(
                context, queue, &ld_values, buffer_rw,
                |int_val| int_val as opencl3::types::cl_uint,
                |double_val| double_val as opencl3::types::cl_uint,
              )?)
            );
          }
          "ulong" => {
            kernel_data.push(
              structs::CL_TaggedArgument::Uint64Buffer(
                write_values_to_cl_buffer::<opencl3::types::cl_ulong>(
                context, queue, &ld_values, buffer_rw,
                |int_val| int_val as opencl3::types::cl_ulong,
                |double_val| double_val as opencl3::types::cl_ulong,
              )?)
            );
          }

          "char" => {
            kernel_data.push(
              structs::CL_TaggedArgument::Int8Buffer(
                write_values_to_cl_buffer::<opencl3::types::cl_char>(
                context, queue, &ld_values, buffer_rw,
                |int_val| int_val as opencl3::types::cl_char,
                |double_val| double_val as opencl3::types::cl_char,
              )?)
            );
          }
          "short" => {
            kernel_data.push(
              structs::CL_TaggedArgument::Int16Buffer(
                write_values_to_cl_buffer::<opencl3::types::cl_short>(
                context, queue, &ld_values, buffer_rw,
                |int_val| int_val as opencl3::types::cl_short,
                |double_val| double_val as opencl3::types::cl_short,
              )?)
            );
          }
          "int" => {
            kernel_data.push(
              structs::CL_TaggedArgument::Int32Buffer(
                write_values_to_cl_buffer::<opencl3::types::cl_int>(
                context, queue, &ld_values, buffer_rw,
                |int_val| int_val as opencl3::types::cl_int,
                |double_val| double_val as opencl3::types::cl_int,
              )?)
            );
          }
          "long" => {
            kernel_data.push(
              structs::CL_TaggedArgument::Int64Buffer(
                write_values_to_cl_buffer::<opencl3::types::cl_long>(
                context, queue, &ld_values, buffer_rw,
                |int_val| int_val as opencl3::types::cl_long,
                |double_val| double_val as opencl3::types::cl_long,
              )?)
            );
          }

          "float" => {
            kernel_data.push(
              structs::CL_TaggedArgument::FloatBuffer(
                write_values_to_cl_buffer::<opencl3::types::cl_float>(
                context, queue, &ld_values, buffer_rw,
                |int_val| int_val as opencl3::types::cl_float,
                |double_val| double_val as opencl3::types::cl_float,
              )?)
            );
          }
          "double" => {
            kernel_data.push(
              structs::CL_TaggedArgument::DoubleBuffer(
                write_values_to_cl_buffer::<opencl3::types::cl_double>(
                context, queue, &ld_values, buffer_rw,
                |int_val| int_val as opencl3::types::cl_double,
                |double_val| double_val as opencl3::types::cl_double,
              )?)
            );
          }

          unk => {
            println!("Unknown CL Buffer type: {}", unk);
            panic!("Unknown CL Buffer type!");
          }
        }

      }
      else {
        // This is a constant, look up in cl_kernel.data_constants and if not exists lookup in sc
        let mut value: Option<structs::CL_TaggedArgument> = None;

        // Look through args.data_constant
        if value.is_none() {
          for dc in args.data_constant.iter() {
            if dc.name == variable_name {
              value = Some( structs::CL_TaggedArgument::from_value(&dc.value, &type_name) );
            }
          }
        }

        // Look through simcontrol toml file
        if value.is_none() {
          if let Some(val_ref) = sc.data_constants.get(&variable_name) {
            value = Some( structs::CL_TaggedArgument::from_value(val_ref, &type_name) );
          }
        }

        // Look through kernel constants
        if value.is_none() {
          for constant in cl_kernel.data_constants.iter() {
            if constant.name == variable_name {
              // Found it!
              value = Some( structs::CL_TaggedArgument::from_value(&constant.value, &type_name) );
              break;
            }
          }
        }

        match value {
          None => {
            println!("[ ERROR ] Cannot find variable '{}' in simulation control file OR in '{}'. Please define a constant named '{}' or pass a value on the command line like --data-constant {}=<VALUE>", &variable_name, &sc.cl_kernels_file_path.display(), &variable_name, &variable_name);
            panic!("Required constant not found in kernels.toml data_constants, or simcontrol.toml data_constants, or passed as --data-constant VAR=val");
          }
          Some(cl_tagged_value) => {
            kernel_data.push(cl_tagged_value);
          }
        }

      }

    }
  }

  Ok(kernel_data)
}


pub fn ld_data_to_kernel_data_named(
    args: &structs::Args,
    sc: &structs::SimControl,
    ld_data: &ListedData,
    context: &opencl3::context::Context,
    cl_kernel: &structs::CL_Kernel,
    k: &opencl3::kernel::Kernel,
    queue: &opencl3::command_queue::CommandQueue,
    events: &Vec<opencl3::types::cl_event>
  ) -> Result<Vec<structs::CL_NamedTaggedArgument>, Box<dyn std::error::Error>>
{
  let mut kernel_data = vec![];

  let work_size = ld_data.len();
  if let Ok(argc) = k.num_args() {
    for arg_i in 0..argc {
      /*
      println!("args[{}] = {:?}, {:?}, {:?}, {:?}, {:?}", arg_i,
        k.get_arg_address_qualifier(arg_i), k.get_arg_access_qualifier(arg_i), k.get_arg_type_qualifier(arg_i),
        k.get_arg_type_name(arg_i), k.get_arg_name(arg_i)
      );
      */
      let is_pointer = k.get_arg_address_qualifier(arg_i)? == 4507;
      let is_constant = k.get_arg_type_qualifier(arg_i)? == 1;
      let type_name = k.get_arg_type_name(arg_i)?;
      let type_name = type_name.trim_end_matches('*'); // Types like 'int*' end with a star, which we do not use b/c we have is_pointer.
      let variable_name = k.get_arg_name(arg_i)?; //.unwrap_or(String::new());
      let variable_name_lowercase = variable_name.to_lowercase();
      let variable_name_uppercase = variable_name.to_uppercase();

      if is_pointer {

        // Lookup data in ld_data w/ fuzzy string matching from all records;
        // We must allocate a [T] because of the signature required by enqueue_write_buffer.
        // Because our goal is to hold massive quantities of data, we limit the buffer to some moderate stack-sized value and loop over it w/ blocking CL writes.

        let mut ld_values: Vec<structs::Value> = vec![];

        for record in ld_data.iter() {
          if let Some(val) = record.get(&variable_name) {
            ld_values.push(val.clone());
          }
          else if let Some(val) = record.get(&variable_name_lowercase) {
            ld_values.push(val.clone());
          }
          else if let Some(val) = record.get(&variable_name_uppercase) {
            ld_values.push(val.clone());
          }
          else {
            if args.verbose > 1 {
              println!("[ Warning ] Missing value for simulation data column {}, 0.0 will be used for this record.", variable_name);
            }
            ld_values.push(structs::Value::Integer(0)); // Default value regardless of type is 0, b/c we allow ld_values to contain different types & unify later
          }
        }

        let buffer_rw = if is_constant { structs::RWColumn::Read(String::new()) } else { structs::RWColumn::Write(String::new()) };

        // Now we match on the CL target type & call into the generic write_values_to_cl_buffer helper routine.
        match type_name {
          "uchar" => {
            kernel_data.push(structs::CL_NamedTaggedArgument::new(
              variable_name.clone(),
              structs::CL_TaggedArgument::Uint8Buffer(
                write_values_to_cl_buffer::<opencl3::types::cl_uchar>(
                context, queue, &ld_values, buffer_rw,
                |int_val| int_val as opencl3::types::cl_uchar,
                |double_val| double_val as opencl3::types::cl_uchar,
              )?)
            ));
          }
          "ushort" => {
            kernel_data.push(structs::CL_NamedTaggedArgument::new(
              variable_name.clone(),
              structs::CL_TaggedArgument::Uint16Buffer(
                write_values_to_cl_buffer::<opencl3::types::cl_ushort>(
                context, queue, &ld_values, buffer_rw,
                |int_val| int_val as opencl3::types::cl_ushort,
                |double_val| double_val as opencl3::types::cl_ushort,
              )?)
            ));
          }
          "uint" => {
            kernel_data.push(
              structs::CL_NamedTaggedArgument::new(
              variable_name.clone(),
              structs::CL_TaggedArgument::Uint32Buffer(
                write_values_to_cl_buffer::<opencl3::types::cl_uint>(
                context, queue, &ld_values, buffer_rw,
                |int_val| int_val as opencl3::types::cl_uint,
                |double_val| double_val as opencl3::types::cl_uint,
              )?)
            ));
          }
          "ulong" => {
            kernel_data.push(structs::CL_NamedTaggedArgument::new(
              variable_name.clone(),
              structs::CL_TaggedArgument::Uint64Buffer(
                write_values_to_cl_buffer::<opencl3::types::cl_ulong>(
                context, queue, &ld_values, buffer_rw,
                |int_val| int_val as opencl3::types::cl_ulong,
                |double_val| double_val as opencl3::types::cl_ulong,
              )?)
            ));
          }

          "char" => {
            kernel_data.push(structs::CL_NamedTaggedArgument::new(
              variable_name.clone(),
              structs::CL_TaggedArgument::Int8Buffer(
                write_values_to_cl_buffer::<opencl3::types::cl_char>(
                context, queue, &ld_values, buffer_rw,
                |int_val| int_val as opencl3::types::cl_char,
                |double_val| double_val as opencl3::types::cl_char,
              )?)
            ));
          }
          "short" => {
            kernel_data.push(structs::CL_NamedTaggedArgument::new(
              variable_name.clone(),
              structs::CL_TaggedArgument::Int16Buffer(
                write_values_to_cl_buffer::<opencl3::types::cl_short>(
                context, queue, &ld_values, buffer_rw,
                |int_val| int_val as opencl3::types::cl_short,
                |double_val| double_val as opencl3::types::cl_short,
              )?)
            ));
          }
          "int" => {
            kernel_data.push(structs::CL_NamedTaggedArgument::new(
              variable_name.clone(),
              structs::CL_TaggedArgument::Int32Buffer(
                write_values_to_cl_buffer::<opencl3::types::cl_int>(
                context, queue, &ld_values, buffer_rw,
                |int_val| int_val as opencl3::types::cl_int,
                |double_val| double_val as opencl3::types::cl_int,
              )?)
            ));
          }
          "long" => {
            kernel_data.push(structs::CL_NamedTaggedArgument::new(
              variable_name.clone(),
              structs::CL_TaggedArgument::Int64Buffer(
                write_values_to_cl_buffer::<opencl3::types::cl_long>(
                context, queue, &ld_values, buffer_rw,
                |int_val| int_val as opencl3::types::cl_long,
                |double_val| double_val as opencl3::types::cl_long,
              )?)
            ));
          }

          "float" => {
            kernel_data.push(structs::CL_NamedTaggedArgument::new(
              variable_name.clone(),
              structs::CL_TaggedArgument::FloatBuffer(
                write_values_to_cl_buffer::<opencl3::types::cl_float>(
                context, queue, &ld_values, buffer_rw,
                |int_val| int_val as opencl3::types::cl_float,
                |double_val| double_val as opencl3::types::cl_float,
              )?)
            ));
          }
          "double" => {
            kernel_data.push(structs::CL_NamedTaggedArgument::new(
              variable_name.clone(),
              structs::CL_TaggedArgument::DoubleBuffer(
                write_values_to_cl_buffer::<opencl3::types::cl_double>(
                context, queue, &ld_values, buffer_rw,
                |int_val| int_val as opencl3::types::cl_double,
                |double_val| double_val as opencl3::types::cl_double,
              )?)
            ));
          }

          unk => {
            println!("Unknown CL Buffer type: {}", unk);
            panic!("Unknown CL Buffer type!");
          }
        }

      }
      else {
        // This is a constant, look up in cl_kernel.data_constants and if not exists lookup in sc
        let mut value: Option<structs::CL_TaggedArgument> = None;

        // Look through args.data_constant
        if value.is_none() {
          for dc in args.data_constant.iter() {
            if dc.name == variable_name {
              value = Some( structs::CL_TaggedArgument::from_value(&dc.value, &type_name) );
            }
          }
        }

        // Look through simcontrol toml file
        if value.is_none() {
          if let Some(val_ref) = sc.data_constants.get(&variable_name) {
            value = Some( structs::CL_TaggedArgument::from_value(val_ref, &type_name) );
          }
        }

        // Look through kernel constants
        if value.is_none() {
          for constant in cl_kernel.data_constants.iter() {
            if constant.name == variable_name {
              // Found it!
              value = Some( structs::CL_TaggedArgument::from_value(&constant.value, &type_name) );
              break;
            }
          }
        }

        match value {
          None => {
            println!("[ ERROR ] Cannot find variable '{}' in simulation control file OR in '{}'. Please define a constant named '{}' or pass a value on the command line like --data-constant {}=<VALUE>", &variable_name, &sc.cl_kernels_file_path.display(), &variable_name, &variable_name);
            panic!("Required constant not found in kernels.toml data_constants, or simcontrol.toml data_constants, or passed as --data-constant VAR=val");
          }
          Some(cl_tagged_value) => {
            kernel_data.push(
              structs::CL_NamedTaggedArgument::new(
                variable_name.clone(), cl_tagged_value
              )
            );
          }
        }

      }

    }
  }

  Ok(kernel_data)
}


fn write_values_to_cl_buffer<T>(
  context: &opencl3::context::Context,
  queue: &opencl3::command_queue::CommandQueue,
  values: &Vec<structs::Value>,
  buffer_rw: structs::RWColumn,
  i64_to_t: impl Fn(i64) -> T,
  f64_to_t: impl Fn(f64) -> T,
)
  -> Result<opencl3::memory::Buffer::<T>, Box<dyn std::error::Error>>
  where T: Copy
{
  const STACK_BUFF_SIZE: usize = 8 * 1024;

  // Allocate buffer of size
  let array_len = values.len();
  let cl_memory_flags = match buffer_rw {
    structs::RWColumn::Read(_)      => opencl3::memory::CL_MEM_READ_ONLY,
    structs::RWColumn::Write(_)     => opencl3::memory::CL_MEM_READ_WRITE,
    structs::RWColumn::ReadWrite(_) => opencl3::memory::CL_MEM_READ_WRITE
  };

  let mut cl_buff = unsafe {
      opencl3::memory::Buffer::<T>::create(&context, cl_memory_flags, array_len, std::ptr::null_mut())?
  };
  let mut cl_buff_write_offset = 0;

  // We write into this over and over again, keeping track of use and making blocking calls to write into cl_buff
  let mut stack_arr: [T; STACK_BUFF_SIZE] = [i64_to_t(0); STACK_BUFF_SIZE];
  let mut stack_arr_write_offset = 0;

  for i in 0..values.len() {

    match values[i] {
        structs::Value::Integer(i) => {
          stack_arr[stack_arr_write_offset] = i64_to_t(i);
        }
        structs::Value::Double(d) => {
          stack_arr[stack_arr_write_offset] = f64_to_t(d);
        }
        structs::Value::String(_) => panic!("Cannot place string value into a CL kernel argument buffer!"),
    }

    stack_arr_write_offset += 1;

    // If we are at the end of the buffer OR at the last value, make blocking write into cl_buff
    if stack_arr_write_offset >= STACK_BUFF_SIZE-1 || i == values.len()-1 {
      let num_items_to_write = if cl_buff_write_offset+STACK_BUFF_SIZE > array_len { array_len - cl_buff_write_offset } else { STACK_BUFF_SIZE };
      let write_event = unsafe { queue.enqueue_write_buffer(&mut cl_buff, opencl3::types::CL_BLOCKING, cl_buff_write_offset, &stack_arr[0..num_items_to_write], &[])? };
      cl_buff_write_offset += STACK_BUFF_SIZE;
    }
  }

  Ok(cl_buff)
}



pub fn kernel_data_update_ld_data(
  cli_args: &structs::Args,
  context: &opencl3::context::Context,
  queue: &opencl3::command_queue::CommandQueue,
  events: &Vec<opencl3::types::cl_event>,
  kernel_data: &Vec<structs::CL_TaggedArgument>,
  kernel_arg_names: &Vec<String>,
  ld_data: &mut ListedData
)
  -> Result<(), Box<dyn std::error::Error>>
{
  use opencl3::memory::ClMem;

  for i in 0..kernel_data.len() {
    let arg_name = &kernel_arg_names[i];
    match &kernel_data[i] {
      structs::CL_TaggedArgument::Uint8Buffer(cl_uchar_buff) => {
        if (cl_uchar_buff.flags().map_err(structs::eloc!())? & opencl3::memory::CL_MEM_READ_WRITE) != 0 {
          // This is a write buffer, so we must read it back out.
          read_values_from_cl_buffer(
            cli_args,
            context, queue, events, cl_uchar_buff, ld_data, arg_name,
            |an_int| an_int as opencl3::types::cl_uchar,
            |a_uchar| structs::Value::Integer(a_uchar as i64)
          ).map_err(structs::eloc!())?;
        }
      }
      structs::CL_TaggedArgument::Uint16Buffer(cl_ushort_buff) => {
        if (cl_ushort_buff.flags().map_err(structs::eloc!())? & opencl3::memory::CL_MEM_READ_WRITE) != 0 {
          // This is a write buffer, so we must read it back out.
          read_values_from_cl_buffer(
            cli_args,
            context, queue, events, cl_ushort_buff, ld_data, arg_name,
            |an_int| an_int as opencl3::types::cl_ushort,
            |a_ushort| structs::Value::Integer(a_ushort as i64)
          ).map_err(structs::eloc!())?;
        }
      }
      structs::CL_TaggedArgument::Uint32Buffer(cl_uint_buff) => {
        if (cl_uint_buff.flags().map_err(structs::eloc!())? & opencl3::memory::CL_MEM_READ_WRITE) != 0 {
          // This is a write buffer, so we must read it back out.
          read_values_from_cl_buffer(
            cli_args,
            context, queue, events, cl_uint_buff, ld_data, arg_name,
            |an_int| an_int as opencl3::types::cl_uint,
            |a_uint| structs::Value::Integer(a_uint as i64)
          ).map_err(structs::eloc!())?;
        }
      }
      structs::CL_TaggedArgument::Uint64Buffer(cl_ulong_buff) => {
        if (cl_ulong_buff.flags().map_err(structs::eloc!())? & opencl3::memory::CL_MEM_READ_WRITE) != 0 {
          // This is a write buffer, so we must read it back out.
          read_values_from_cl_buffer(
            cli_args,
            context, queue, events, cl_ulong_buff, ld_data, arg_name,
            |an_int| an_int as opencl3::types::cl_ulong,
            |a_ulong| structs::Value::Integer(a_ulong.try_into().expect("Failed to convert a u64 to a i64!"))
          ).map_err(structs::eloc!())?;
        }
      }

      structs::CL_TaggedArgument::Int8Buffer(cl_char_buff) => {
        if (cl_char_buff.flags().map_err(structs::eloc!())? & opencl3::memory::CL_MEM_READ_WRITE) != 0 {
          // This is a write buffer, so we must read it back out.
          read_values_from_cl_buffer(
            cli_args,
            context, queue, events, cl_char_buff, ld_data, arg_name,
            |an_int| an_int as opencl3::types::cl_char,
            |a_char| structs::Value::Integer(a_char as i64)
          ).map_err(structs::eloc!())?;
        }
      }
      structs::CL_TaggedArgument::Int16Buffer(cl_short_buff) => {
        if (cl_short_buff.flags().map_err(structs::eloc!())? & opencl3::memory::CL_MEM_READ_WRITE) != 0 {
          // This is a write buffer, so we must read it back out.
          read_values_from_cl_buffer(
            cli_args,
            context, queue, events, cl_short_buff, ld_data, arg_name,
            |an_int| an_int as opencl3::types::cl_short,
            |a_short| structs::Value::Integer(a_short as i64)
          ).map_err(structs::eloc!())?;
        }
      }
      structs::CL_TaggedArgument::Int32Buffer(cl_int_buff) => {
        if (cl_int_buff.flags().map_err(structs::eloc!())? & opencl3::memory::CL_MEM_READ_WRITE) != 0 {
          // This is a write buffer, so we must read it back out.
          read_values_from_cl_buffer(
            cli_args,
            context, queue, events, cl_int_buff, ld_data, arg_name,
            |an_int| an_int as opencl3::types::cl_int,
            |a_int| structs::Value::Integer(a_int as i64)
          ).map_err(structs::eloc!())?;
        }
      }
      structs::CL_TaggedArgument::Int64Buffer(cl_long_buff) => {
        if (cl_long_buff.flags().map_err(structs::eloc!())? & opencl3::memory::CL_MEM_READ_WRITE) != 0 {
          // This is a write buffer, so we must read it back out.
          read_values_from_cl_buffer(
            cli_args,
            context, queue, events, cl_long_buff, ld_data, arg_name,
            |an_int| an_int as opencl3::types::cl_long,
            |a_long| structs::Value::Integer(a_long as i64)
          ).map_err(structs::eloc!())?;
        }
      }


      structs::CL_TaggedArgument::FloatBuffer(cl_float_buff) => {
        if (cl_float_buff.flags().map_err(structs::eloc!())? & opencl3::memory::CL_MEM_READ_WRITE) != 0 {
          // This is a write buffer, so we must read it back out.
          read_values_from_cl_buffer(
            cli_args,
            context, queue, events, cl_float_buff, ld_data, arg_name,
            |an_int| an_int as opencl3::types::cl_float,
            |a_float| structs::Value::Double(a_float as f64)
          ).map_err(structs::eloc!())?;
        }
      }
      structs::CL_TaggedArgument::DoubleBuffer(cl_double_buff) => {
        if (cl_double_buff.flags().map_err(structs::eloc!())? & opencl3::memory::CL_MEM_READ_WRITE) != 0 {
          // This is a write buffer, so we must read it back out.
          read_values_from_cl_buffer(
            cli_args,
            context, queue, events, cl_double_buff, ld_data, arg_name,
            |an_int| an_int as opencl3::types::cl_double,
            |a_double| structs::Value::Double(a_double as f64)
          ).map_err(structs::eloc!())?;
        }
      }

      structs::CL_TaggedArgument::Uint8(unused) => { /* NOP */ }
      structs::CL_TaggedArgument::Uint16(unused) => { /* NOP */ }
      structs::CL_TaggedArgument::Uint32(unused) => { /* NOP */ }
      structs::CL_TaggedArgument::Uint64(unused) => { /* NOP */ }
      structs::CL_TaggedArgument::Int8(unused) => { /* NOP */ }
      structs::CL_TaggedArgument::Int16(unused) => { /* NOP */ }
      structs::CL_TaggedArgument::Int32(unused) => { /* NOP */ }
      structs::CL_TaggedArgument::Int64(unused) => { /* NOP */ }
      structs::CL_TaggedArgument::Float(unused) => { /* NOP */ }
      structs::CL_TaggedArgument::Double(unused) => { /* NOP */ }

      /*unhandled => {
        println!("Unhandled variant of structs::CL_TaggedArgument: {:?}", unhandled);
        panic!("");
      }*/
    }
  }


  Ok(())
}

pub fn kernel_data_update_ld_data_named(cli_args: &structs::Args,
  context: &opencl3::context::Context,
  queue: &opencl3::command_queue::CommandQueue,
  events: &Vec<opencl3::types::cl_event>,
  kernel_data: &Vec<structs::CL_NamedTaggedArgument>,
  ld_data: &mut ListedData
)
  -> Result<(), Box<dyn std::error::Error>>
{
  use opencl3::memory::ClMem;


  for i in 0..kernel_data.len() {
    let arg_name = &kernel_data[i].name;
    match kernel_data[i].tagged_argument.borrow() {
      structs::CL_TaggedArgument::Uint8Buffer(cl_uchar_buff) => {
        if (cl_uchar_buff.flags().map_err(structs::eloc!())? & opencl3::memory::CL_MEM_READ_WRITE) != 0 {
          // This is a write buffer, so we must read it back out.
          read_values_from_cl_buffer(
            cli_args,
            context, queue, events, cl_uchar_buff, ld_data, &arg_name,
            |an_int| an_int as opencl3::types::cl_uchar,
            |a_uchar| structs::Value::Integer(a_uchar as i64)
          ).map_err(structs::eloc!())?;
        }
      }
      structs::CL_TaggedArgument::Uint16Buffer(cl_ushort_buff) => {
        if (cl_ushort_buff.flags().map_err(structs::eloc!())? & opencl3::memory::CL_MEM_READ_WRITE) != 0 {
          // This is a write buffer, so we must read it back out.
          read_values_from_cl_buffer(
            cli_args,
            context, queue, events, cl_ushort_buff, ld_data, &arg_name,
            |an_int| an_int as opencl3::types::cl_ushort,
            |a_ushort| structs::Value::Integer(a_ushort as i64)
          ).map_err(structs::eloc!())?;
        }
      }
      structs::CL_TaggedArgument::Uint32Buffer(cl_uint_buff) => {
        if (cl_uint_buff.flags().map_err(structs::eloc!())? & opencl3::memory::CL_MEM_READ_WRITE) != 0 {
          // This is a write buffer, so we must read it back out.
          read_values_from_cl_buffer(
            cli_args,
            context, queue, events, cl_uint_buff, ld_data, &arg_name,
            |an_int| an_int as opencl3::types::cl_uint,
            |a_uint| structs::Value::Integer(a_uint as i64)
          ).map_err(structs::eloc!())?;
        }
      }
      structs::CL_TaggedArgument::Uint64Buffer(cl_ulong_buff) => {
        if (cl_ulong_buff.flags().map_err(structs::eloc!())? & opencl3::memory::CL_MEM_READ_WRITE) != 0 {
          // This is a write buffer, so we must read it back out.
          read_values_from_cl_buffer(
            cli_args,
            context, queue, events, cl_ulong_buff, ld_data, &arg_name,
            |an_int| an_int as opencl3::types::cl_ulong,
            |a_ulong| structs::Value::Integer(a_ulong.try_into().expect("Failed to convert a u64 to a i64!"))
          ).map_err(structs::eloc!())?;
        }
      }

      structs::CL_TaggedArgument::Int8Buffer(cl_char_buff) => {
        if (cl_char_buff.flags().map_err(structs::eloc!())? & opencl3::memory::CL_MEM_READ_WRITE) != 0 {
          // This is a write buffer, so we must read it back out.
          read_values_from_cl_buffer(
            cli_args,
            context, queue, events, cl_char_buff, ld_data, &arg_name,
            |an_int| an_int as opencl3::types::cl_char,
            |a_char| structs::Value::Integer(a_char as i64)
          ).map_err(structs::eloc!())?;
        }
      }
      structs::CL_TaggedArgument::Int16Buffer(cl_short_buff) => {
        if (cl_short_buff.flags().map_err(structs::eloc!())? & opencl3::memory::CL_MEM_READ_WRITE) != 0 {
          // This is a write buffer, so we must read it back out.
          read_values_from_cl_buffer(
            cli_args,
            context, queue, events, cl_short_buff, ld_data, &arg_name,
            |an_int| an_int as opencl3::types::cl_short,
            |a_short| structs::Value::Integer(a_short as i64)
          ).map_err(structs::eloc!())?;
        }
      }
      structs::CL_TaggedArgument::Int32Buffer(cl_int_buff) => {
        if (cl_int_buff.flags().map_err(structs::eloc!())? & opencl3::memory::CL_MEM_READ_WRITE) != 0 {
          // This is a write buffer, so we must read it back out.
          read_values_from_cl_buffer(
            cli_args,
            context, queue, events, cl_int_buff, ld_data, &arg_name,
            |an_int| an_int as opencl3::types::cl_int,
            |a_int| structs::Value::Integer(a_int as i64)
          ).map_err(structs::eloc!())?;
        }
      }
      structs::CL_TaggedArgument::Int64Buffer(cl_long_buff) => {
        if (cl_long_buff.flags().map_err(structs::eloc!())? & opencl3::memory::CL_MEM_READ_WRITE) != 0 {
          // This is a write buffer, so we must read it back out.
          read_values_from_cl_buffer(
            cli_args,
            context, queue, events, cl_long_buff, ld_data, &arg_name,
            |an_int| an_int as opencl3::types::cl_long,
            |a_long| structs::Value::Integer(a_long as i64)
          ).map_err(structs::eloc!())?;
        }
      }


      structs::CL_TaggedArgument::FloatBuffer(cl_float_buff) => {
        if (cl_float_buff.flags().map_err(structs::eloc!())? & opencl3::memory::CL_MEM_READ_WRITE) != 0 {
          // This is a write buffer, so we must read it back out.
          read_values_from_cl_buffer(
            cli_args,
            context, queue, events, cl_float_buff, ld_data, &arg_name,
            |an_int| an_int as opencl3::types::cl_float,
            |a_float| structs::Value::Double(a_float as f64)
          ).map_err(structs::eloc!())?;
        }
      }
      structs::CL_TaggedArgument::DoubleBuffer(cl_double_buff) => {
        if (cl_double_buff.flags().map_err(structs::eloc!())? & opencl3::memory::CL_MEM_READ_WRITE) != 0 {
          // This is a write buffer, so we must read it back out.
          read_values_from_cl_buffer(
            cli_args,
            context, queue, events, cl_double_buff, ld_data, &arg_name,
            |an_int| an_int as opencl3::types::cl_double,
            |a_double| structs::Value::Double(a_double as f64)
          ).map_err(structs::eloc!())?;
        }
      }

      structs::CL_TaggedArgument::Uint8(unused) => { /* NOP */ }
      structs::CL_TaggedArgument::Uint16(unused) => { /* NOP */ }
      structs::CL_TaggedArgument::Uint32(unused) => { /* NOP */ }
      structs::CL_TaggedArgument::Uint64(unused) => { /* NOP */ }
      structs::CL_TaggedArgument::Int8(unused) => { /* NOP */ }
      structs::CL_TaggedArgument::Int16(unused) => { /* NOP */ }
      structs::CL_TaggedArgument::Int32(unused) => { /* NOP */ }
      structs::CL_TaggedArgument::Int64(unused) => { /* NOP */ }
      structs::CL_TaggedArgument::Float(unused) => { /* NOP */ }
      structs::CL_TaggedArgument::Double(unused) => { /* NOP */ }

      /*unhandled => {
        println!("Unhandled variant of structs::CL_TaggedArgument: {:?}", unhandled);
        panic!("");
      }*/
    }
  }


  Ok(())
}


fn read_values_from_cl_buffer<T>(
  cli_args: &structs::Args,
  context: &opencl3::context::Context,
  queue: &opencl3::command_queue::CommandQueue,
  events: &Vec<opencl3::types::cl_event>,
  cl_values: &opencl3::memory::Buffer<T>,
  ld_data: &mut ListedData,
  ld_field_name: &str,
  i64_to_t: impl Fn(i64) -> T,
  t_to_val: impl Fn(T) -> structs::Value,
)
  -> Result<(), Box<dyn std::error::Error>>
  where T: Copy
{
  use opencl3::memory::ClMem;

  const STACK_BUFF_SIZE: usize = 8 * 1024;

  // Allocate buffer of size
  let array_len = cl_values.size().map_err(structs::eloc!())? / std::mem::size_of::<T>();

  let mut ld_data_write_offset = 0;
  let mut cl_buff_read_offset = 0;

  // We read into this over and over again, keeping track of use and making blocking calls to read from cl_values
  let mut stack_arr: [T; STACK_BUFF_SIZE] = [i64_to_t(0); STACK_BUFF_SIZE];
  let mut stack_arr_write_offset = 0;


  for i in (0..array_len).step_by(STACK_BUFF_SIZE) {
    // Read the next STACK_BUFF_SIZE items up to a max of aray_size
    if cli_args.verbose > 1 {
      eprintln!("i = {} cl_buff_read_offset = {}", i, cl_buff_read_offset);
    }
    let num_items_read = if cl_buff_read_offset+STACK_BUFF_SIZE > array_len { array_len - cl_buff_read_offset } else { STACK_BUFF_SIZE };
    let unused_read_event = unsafe { queue.enqueue_read_buffer(cl_values, opencl3::types::CL_BLOCKING, cl_buff_read_offset, &mut stack_arr[0..num_items_read], &events).map_err(structs::eloc!())? };
    cl_buff_read_offset += STACK_BUFF_SIZE;

    for j in 0..num_items_read {
      if ld_data[ld_data_write_offset].contains_key(ld_field_name) {
        *ld_data[ld_data_write_offset].get_mut(ld_field_name).expect("Safety: we checked contains_key upstairs") = t_to_val(stack_arr[j])
      }
      else {
        ld_data[ld_data_write_offset].insert(ld_field_name.to_string(), t_to_val(stack_arr[j]) );
      }
      ld_data_write_offset += 1;
    }
  }

  Ok(())
}





pub fn duration_to_display_str(d: &std::time::Duration) -> String {
  let total_millis = d.as_millis();
  let ms = total_millis % 1000;
  let s = (total_millis / 1000) % 60;
  let m = (total_millis / (1000 * 60)) % 60;
  let h = total_millis / (1000 * 60 * 60) /* % 24 */;
  if h > 0 {
    format!("{:0>2}h {:0>2}m {:0>2}s {:0>3}ms", h, m, s, ms)
  }
  else if m > 0 {
    format!("{:0>2}m {:0>2}s {:0>3}ms", m, s, ms)
  }
  else if s > 0 {
    format!("{:0>2}s {:0>3}ms", s, ms)
  }
  else {
    format!("{:0>3}ms", ms)
  }
}



pub fn trim_completed_events(
  cli_args: &structs::Args,
  sim_events: &mut Vec<opencl3::event::Event>,
  sim_events_cl: &mut Vec<opencl3::types::cl_event>,
) -> Result<(), Box<dyn std::error::Error>>
{

  let mut events_to_remove = vec![];

  for evt_i in 0..sim_events.len() {
    match sim_events[evt_i].command_execution_status() {
      Ok(status) => {
        if status.0  == opencl3::event::CL_COMPLETE {
          events_to_remove.push(evt_i);
        }
      }
      Err(e) => {
        if cli_args.verbose > 0 {
          eprintln!("{:?}", e);
        }
      }
    }
  }

  // We special case if ALL events are complete and simply truncate both lists
  if sim_events.len() == events_to_remove.len() {
    sim_events_cl.clear();
    sim_events.clear();
    return Ok(());
  }

  // Measuring done, remove all indexes in events_to_remove;
  // swap_remove does not alter indexes of items at X+1 -> N,
  // but it does invalidate the index of the last element!
  // Because of this we do NOT remove indicies > sim_events.len() - events_to_remove.len()
  // Elements marked for removal at the _end_ are expected to become events at the beginning + removed in later passes.
  let last_allowed_idx_to_rm = std::cmp::max(
    sim_events.len() - events_to_remove.len(),
    1
  );

  if cli_args.verbose > 1 {
    eprintln!("last_allowed_idx_to_rm = {}", last_allowed_idx_to_rm);
    eprintln!("sim_events_cl = {:?}", sim_events_cl);
    eprintln!("events_to_remove = {:?}", events_to_remove);
  }
  for idx_to_rm in events_to_remove.iter() {
    let idx_to_rm = *idx_to_rm;
    if idx_to_rm < last_allowed_idx_to_rm {
      sim_events_cl.swap_remove(idx_to_rm);
      sim_events.swap_remove(idx_to_rm);
    }
  }


  Ok(())
}


