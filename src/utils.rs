
use std::collections::HashMap;

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

pub fn inplace_update_simcontrol_from_args(simcontrol: &mut structs::SimControl, cli_args: &structs::Args) {
  if let Some(num_steps) = &cli_args.num_steps {
    println!("Overriding simcontrol num_steps={} with cli arg value ={}", simcontrol.num_steps, num_steps);
    simcontrol.num_steps = *num_steps;
  }

  if let Some(preferred_gpu_name) = &cli_args.preferred_gpu_name {
    println!("Overriding simcontrol preferred_gpu_name={} with cli arg value ={}", simcontrol.preferred_gpu_name, preferred_gpu_name);
    simcontrol.preferred_gpu_name = preferred_gpu_name.to_string();
  }

  if let Some(data_file_path) = &cli_args.data_file_path {
    println!("Overriding simcontrol data_file_path={} with cli arg value ={}", simcontrol.data_file_path.display(), data_file_path.display());
    simcontrol.data_file_path = data_file_path.clone();
  }

  if let Some(cl_kernels_file_path) = &cli_args.cl_kernels_file_path {
    println!("Overriding simcontrol cl_kernels_file_path={} with cli arg value ={}", simcontrol.cl_kernels_file_path.display(), cl_kernels_file_path.display());
    simcontrol.cl_kernels_file_path = cl_kernels_file_path.clone();
  }

  if let Some(cl_kernels_file_path) = &cli_args.cl_kernels_file_path {
    println!("Overriding simcontrol cl_kernels_file_path={} with cli arg value ={}", simcontrol.cl_kernels_file_path.display(), cl_kernels_file_path.display());
    simcontrol.cl_kernels_file_path = cl_kernels_file_path.clone();
  }

  if let Some(gis_x_attr_name) = &cli_args.gis_x_attr_name {
    println!("Overriding simcontrol gis_x_attr_name={} with cli arg value ={}", simcontrol.gis_x_attr_name, gis_x_attr_name);
    simcontrol.gis_x_attr_name = gis_x_attr_name.to_string();
  }

  if let Some(gis_y_attr_name) = &cli_args.gis_y_attr_name {
    println!("Overriding simcontrol gis_y_attr_name={} with cli arg value ={}", simcontrol.gis_y_attr_name, gis_y_attr_name);
    simcontrol.gis_y_attr_name = gis_y_attr_name.to_string();
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
      let is_pointer = k.get_arg_address_qualifier(arg_i).unwrap_or(0) == 4507;
      let is_constant = k.get_arg_type_qualifier(arg_i).unwrap_or(0) == 1;
      let type_name = k.get_arg_type_name(arg_i).unwrap_or(String::new());
      let type_name = type_name.trim_end_matches('*'); // Types like 'int*' end with a star, which we do not use b/c we have is_pointer.
      let variable_name = k.get_arg_name(arg_i).unwrap_or(String::new());
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
            if args.verbose > 0 {
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
        for constant in cl_kernel.data_constants.iter() {
          if constant.name == variable_name {
            // Found it!
            value = Some( structs::CL_TaggedArgument::from_value(&constant.value, &type_name) );
            break;
          }
        }
        if value.is_none() {
          // Look through sc
          if let Some(val_ref) = sc.data_constants.get(&variable_name) {
            value = Some( structs::CL_TaggedArgument::from_value(val_ref, &type_name) );
          }
        }
        match value {
          None => {
            println!("[ ERROR ] Cannot find variable {} simulation control file OR in {}. Please define a constant named {}", &variable_name, &sc.cl_kernels_file_path.display(), &variable_name);
            panic!("Required constant not found in kernels.toml data_constants or simcontrol.toml data_constants");
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
  let array_size = values.len();
  let cl_memory_flags = match buffer_rw {
    structs::RWColumn::Read(_)      => opencl3::memory::CL_MEM_READ_ONLY,
    structs::RWColumn::Write(_)     => opencl3::memory::CL_MEM_READ_WRITE,
    structs::RWColumn::ReadWrite(_) => opencl3::memory::CL_MEM_READ_WRITE
  };

  let mut cl_buff = unsafe {
      opencl3::memory::Buffer::<T>::create(&context, cl_memory_flags, array_size, std::ptr::null_mut())?
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
      let write_event = unsafe { queue.enqueue_write_buffer(&mut cl_buff, opencl3::types::CL_BLOCKING, 0, &stack_arr, &[])? };
    }
  }




  Ok(cl_buff)
}



pub fn kernel_data_update_ld_data(kernel_data: &Vec<structs::CL_TaggedArgument>, ld_data: &ListedData)
  -> Result<(), Box<dyn std::error::Error>>
{
  println!("TODO implement kernel_data_update_ld_data");

  Ok(())
}

