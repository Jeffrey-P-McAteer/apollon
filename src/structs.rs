
use crate::utils;

use std::collections::HashMap;


#[derive(Debug, clap::Parser)]
pub struct Args {
    /// A data file (.toml, .json, etc.) containing simulation configuration data.
    pub simcontrol_file_path: std::path::PathBuf,

    /// A data file (.csv, .json, etc.) containing T=0 data for the simulation.
    #[arg(short, long)]
    pub input_data_file_path: Option<std::path::PathBuf>,

    /// A data file (.csv, .json, etc.) path which will have T=<num_steps> data from the simulation written to it
    #[arg(short, long)]
    pub output_data_file_path: Option<std::path::PathBuf>,

    /// A data file (.toml) containing OpenCL kernels to be executed,
    /// and which is expected to supply the delta_file_path with functions to use.
    #[arg(short, long)]
    pub cl_kernels_file_path: Option<std::path::PathBuf>,

    /// Path to animated .gif file which will contain saved off GIS frame data every capture-step-period simulation steps
    #[arg(long)]
    pub output_animation_file_path: Option<std::path::PathBuf>,

    /// Width in pixels of output animation
    #[arg(long)]
    pub output_animation_width: Option<u32>,

    /// Height in pixels of output animation
    #[arg(long)]
    pub output_animation_height: Option<u32>,

    /// Animation frame delay in milliseconds
    #[arg(long)]
    pub output_animation_frame_delay: Option<u32>,

    /// Number of simulation steps to run
    #[arg(short, long)]
    pub num_steps: Option<u64>,

    /// Every N steps a single animation frame is recorded; this allows high-resolution simulations to run w/o creating massive+slow animation files by capturing eg every 10 simulation steps.
    #[arg(long)]
    pub capture_step_period: Option<u64>,


    /// Preferred GPU name to use. Pass "LIST" to list all GPUs detected on this system.
    #[arg(short, long)]
    pub preferred_gpu_name: Option<String>,

    /// Which attribute in delta_file_path holds the item's X position?
    #[arg( long)]
    pub gis_x_attr_name: Option<String>,

    /// Which attribute in delta_file_path holds the item's Y position?
    #[arg(long)]
    pub gis_y_attr_name: Option<String>,

    /// Which attribute in delta_file_path holds the item's Name?
    #[arg(long)]
    pub gis_name_attr: Option<String>,

    /// Which attribute in delta_file_path holds the item's Color if drawn on a graph?
    #[arg(long)]
    pub gis_color_attr: Option<String>,

    /// Specify data constants such as, which override simcontrol_file_path, which override cl_kernels_file_path.
    /// This is best used for briefly testing 1 different value among a set of many constants for a simulation.
    /// Example syntax: --data-constant SIM_VAR_NAME=5.23
    #[arg(short, long, value_parser = NamedDataConstant::from_str )]
    pub data_constant: Vec<NamedDataConstant>,

    /// Amount of verbosity in printed status messages; can be specified multiple times (ie "-v", "-vv", "-vvv" for greater verbosity)
    #[arg(short = 'v', long, action = clap::ArgAction::Count)]
    pub verbose: u8,

}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct NamedDataConstant {
  pub name: String,
  pub value: Value,
}

impl NamedDataConstant {
  pub fn from_str(s: &str) -> Result<NamedDataConstant, Box<dyn std::error::Error + Send + Sync + 'static>> {
    match s.split_once('=') {
      Some((key, value)) => {
        Ok(
          Self {
            name: key.into(),
            value: Value::from_str(value)
          }
        )
      }
      None => {
        Err("Bad format for variable! Expected format is NAME=<number>, where <number> is an arabic numeric".into())
      }
    }
  }
}



#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SimControl_file { // utility to allow us to specify name of value
  pub simulation: SimControl,
  pub data_constants: HashMap<String, Value>,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SimControl {
    /// A data file (.csv, .json, etc.) containing T=0 data for the simulation.
    pub input_data_file_path: std::path::PathBuf,

    /// A data file (.csv, .json, etc.) path which will have T=<num_steps> data from the simulation written to it
    #[serde(default = "serde_default_pathbuf_devnull")]
    pub output_data_file_path: std::path::PathBuf,

    /// A data file (.toml) containing OpenCL kernels to be executed,
    /// and which is expected to supply the delta_file_path with functions to use.
    pub cl_kernels_file_path: std::path::PathBuf,

    #[serde(default = "serde_default_pathbuf_devnull")]
    pub output_animation_file_path: std::path::PathBuf,

    /// Width in pixels of output animation
    #[serde(default = "serde_default_output_animation_width")]
    pub output_animation_width: u32,

    /// Height in pixels of output animation
    #[serde(default = "serde_default_output_animation_height")]
    pub output_animation_height: u32,

    /// Animation frame delay in milliseconds
    #[serde(default = "serde_default_output_animation_frame_delay")]
    pub output_animation_frame_delay: u32,


    /// Number of simulation steps to run
    #[serde(default = "serde_default_num_steps")]
    pub num_steps: u64,

    /// Every N steps a single animation frame is recorded; this allows high-resolution simulations to run w/o creating massive+slow animation files by capturing eg every 10 simulation steps.
    #[serde(default = "serde_default_capture_step_period")]
    pub capture_step_period: u64,

    /// Preferred GPU name to use. Pass "LIST" to list all GPUs detected on this system.
    #[serde(default = "serde_empty_string")]
    pub preferred_gpu_name: String,

    #[serde(default = "serde_default_gis_x_attr_name")]
    pub gis_x_attr_name: String,
    #[serde(default = "serde_default_gis_y_attr_name")]
    pub gis_y_attr_name: String,

    #[serde(default = "serde_default_gis_name_attr")]
    pub gis_name_attr: String,

    /// Which attribute in delta_file_path holds the item's Color if drawn on a graph?
    #[serde(default = "serde_default_gis_color_attr")]
    pub gis_color_attr: String,

    // If not specified under [simulation], these are copied in from SimControl_file
    #[serde(default = "serde_default_value_map")]
    pub data_constants: HashMap<String, Value>,

}

fn serde_empty_string()              -> String { String::new() }
fn serde_default_num_steps()         -> u64    { 64 }
fn serde_default_gis_x_attr_name()   -> String { "X".to_string() }
fn serde_default_gis_y_attr_name()   -> String { "Y".to_string() }
fn serde_default_gis_name_attr()     -> String { "".to_string() }
fn serde_default_gis_color_attr()    -> String { "".to_string() }
fn serde_default_column_types()      -> HashMap<String, ValueType> { HashMap::<String, ValueType>::new() }
fn serde_default_value_map()         -> HashMap<String, Value> { HashMap::<String, Value>::new() }

#[cfg(target_os = "windows")]
fn serde_default_pathbuf_devnull()   -> std::path::PathBuf { "NUL".into() }
#[cfg(not(target_os = "windows"))]
fn serde_default_pathbuf_devnull()   -> std::path::PathBuf { "/dev/null".into() }

fn serde_default_capture_step_period() -> u64 { 10 }

fn serde_default_output_animation_width()   -> u32 { 1280 }
fn serde_default_output_animation_height()  -> u32 { 960 }
fn serde_default_output_animation_frame_delay()  -> u32 { 250 }





#[derive(Default, Debug, Clone, serde::Serialize)]
#[serde(untagged)]
pub enum ValueType {
  Uint8,
  Uint16,
  Uint32,
  Uint64,

  Int8,
  Int16,
  Int32,
  Int64,

  Float32,
  #[default]
  Float64,
}

impl ValueType {
  pub fn maybe_from_str(str_val: &str) -> Option<ValueType> {
    let str_val = str_val.to_lowercase();
    match str_val.as_str() {
      "uint8"  => Some(ValueType::Uint8),
      "uint16" => Some(ValueType::Uint16),
      "uint32" => Some(ValueType::Uint32),
      "uint64" => Some(ValueType::Uint64),

      "u8"  => Some(ValueType::Uint8),
      "u16" => Some(ValueType::Uint16),
      "u32" => Some(ValueType::Uint32),
      "u64" => Some(ValueType::Uint64),

      "int8"  => Some(ValueType::Int8),
      "int16" => Some(ValueType::Int16),
      "int32" => Some(ValueType::Int32),
      "int64" => Some(ValueType::Int64),

      "i8"  => Some(ValueType::Int8),
      "i16" => Some(ValueType::Int16),
      "i32" => Some(ValueType::Int32),
      "i64" => Some(ValueType::Int64),

      "float"   => Some(ValueType::Float32),
      "float32" => Some(ValueType::Float32),
      "f32"     => Some(ValueType::Float32),

      "double"   => Some(ValueType::Float32),
      "f64"      => Some(ValueType::Float32),

      unk_val => {
        None
      },
    }
  }
}


impl<'de> serde::Deserialize<'de> for ValueType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct ValueTypeVisitor;
        impl<'de> serde::de::Visitor<'de> for ValueTypeVisitor {
            type Value = ValueType;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "A String value of u8, u16, u32, u64, i8, i16, i32, i64, f32, f64 or any of their aliases: uint8, uint16, uint32, uint64, int8, int16, int32, int64, float, float32 or double.")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
              E: serde::de::Error,
            {
              if let Some(val_type) = ValueType::maybe_from_str(v) {
                Ok(val_type)
              }
              else {
                Err(serde::de::Error::invalid_value(serde::de::Unexpected::Str(v), &self))
              }
            }
        }

        deserializer.deserialize_any(ValueTypeVisitor)
    }
}












#[derive(Debug, Clone, serde::Serialize)]
#[serde(untagged)]
pub enum Value {
  Integer(i64),
  Double(f64),
  String(String),
}

impl Value {
  pub fn from_str(str_val: &str) -> Value {
    if let Ok(i64_val) = str_val.parse::<i64>() {
      Value::Integer(i64_val)
    }
    else if let Ok(f64_val) = str_val.parse::<f64>() {
      Value::Double(f64_val)
    }
    else {
      Value::String(str_val.to_string())
    }
  }
  pub fn to_i64(&self) -> Result<i64, Box<dyn std::error::Error>> {
    match self {
      Value::Integer(i) => Ok(*i),
      Value::Double(f) =>  Ok(f.round() as i64),
      Value::String(s) =>  Ok(s.parse::<i64>()?),
    }
  }
  pub fn to_i32(&self) -> Result<i32, Box<dyn std::error::Error>> {
    match self {
      Value::Integer(i) => Ok(*i as i32),
      Value::Double(f) =>  Ok(f.round() as i32),
      Value::String(s) =>  Ok(s.parse::<i32>()?),
    }
  }
  pub fn to_f64(&self) -> Result<f64, Box<dyn std::error::Error>> {
    match self {
      Value::Integer(i) => Ok(*i as f64),
      Value::Double(f) =>  Ok(*f),
      Value::String(s) =>  Ok(s.parse::<f64>()?),
    }
  }
  pub fn to_f32(&self) -> Result<f32, Box<dyn std::error::Error>> {
    match self {
      Value::Integer(i) => Ok(*i as f32),
      Value::Double(f) =>  Ok(*f as f32),
      Value::String(s) =>  Ok(s.parse::<f32>()?),
    }
  }
  pub fn to_string(&self) -> String {
    match self {
      Value::Integer(i) => format!("{}", i),
      Value::Double(f) =>  format!("{}", f),
      Value::String(s) =>  s.clone(),
    }
  }
}

impl std::hash::Hash for Value {
  fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
    match self {
      Value::Integer(_i64) => {
        _i64.hash(state);
      }
      Value::Double(_f64) => {
        //_f64.hash(state);
        if _f64.is_nan() {
          (0 as i64).hash(state);
        }
        else {
          let large_num = (_f64 * 1_000_000_000.0) as i64;
          large_num.hash(state);
        }
      }
      Value::String(s) => {
        s.hash(state);
      }
    }
  }
}


impl<'de> serde::Deserialize<'de> for Value {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct ValueVisitor;
        impl<'de> serde::de::Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "Any Integer, Double, or String value")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            {
                Ok( Value::from_str(v) )
            }

            fn  visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            {
                Ok( Value::Integer(v) )
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            {
                Ok( Value::Double(v) )
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}



#[derive(Debug, Clone, serde::Serialize)]
#[serde(untagged)]
pub enum RWColumn {
  Read(String),
  Write(String),
  ReadWrite(String),
}


impl RWColumn {
  pub fn from_str(str_val: &str) -> RWColumn {
    if str_val.starts_with("r:") {
      RWColumn::Read( str_val.strip_prefix("r:").unwrap_or(str_val).to_string() )
    }
    else if str_val.starts_with("w:") {
      RWColumn::Write( str_val.strip_prefix("w:").unwrap_or(str_val).to_string() )
    }
    else { // Assume "rw:" or similar
      RWColumn::ReadWrite( str_val.strip_prefix("rw:").unwrap_or(str_val).to_string() )
    }
  }
}


impl<'de> serde::Deserialize<'de> for RWColumn {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct ValueVisitor;
        impl<'de> serde::de::Visitor<'de> for ValueVisitor {
            type Value = RWColumn;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "String beginning with 'r', or 'w', or 'rw' like 'r:<column name>' or 'w:<column name>' or 'rw:<column name>'")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            {
                Ok( RWColumn::from_str(v) )
            }

        }

        deserializer.deserialize_any(ValueVisitor)
    }
}




#[derive(Debug, Clone, serde::Serialize)]
pub struct DataConstantValue {
  pub name: String,
  pub v_type: ValueType,
  pub value: Value,
}


impl<'de> serde::Deserialize<'de> for DataConstantValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct ValueVisitor;
        impl<'de> serde::de::Visitor<'de> for ValueVisitor {
            type Value = DataConstantValue;

            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "TODO docs")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
              where
              A: serde::de::SeqAccess<'de>,

            {
              if let (Some(name), Some(v_type), Some(value)) = (seq.next_element::<String>()?, seq.next_element::<ValueType>()?, seq.next_element::<Value>()?) {
                Ok(DataConstantValue {
                  name: name,
                  v_type: v_type,
                  value: value,
                })
              }
              else {
                panic!("TODO")
              }
            }

        }

        deserializer.deserialize_any(ValueVisitor)
    }
}



#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CL_Kernels {
  pub kernel: Vec<CL_Kernel>,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CL_Kernel {
  pub name: String,

  /*
  #[serde(default = "serde_default_column_types")]
  pub column_types: HashMap<String, ValueType>,

  #[serde(default = "serde_default_data_columns_processed")]
  pub data_columns_processed: Vec<RWColumn>,
  */

  #[serde(default = "serde_default_colmap")]
  pub colmap: HashMap<String, String>,

  /// Contains the same keys as colmap; is expected to be constructed at run-time by parsing colmap and the kernel source code.
  #[serde(default = "serde_default_typemap")]
  pub typemap: HashMap<String, ValueType>,

  #[serde(default = "serde_default_data_constants")]
  pub data_constants: Vec<DataConstantValue>,

  pub source: String,

  /// This should be a single string containing flags listed in https://registry.khronos.org/OpenCL/specs/3.0-unified/html/OpenCL_API.html#compiler-options
  #[serde(default = "serde_empty_string")]
  pub cl_program_compiler_options: String,

  #[serde(skip_serializing, skip_deserializing)]
  pub cl_device_program: Option<opencl3::program::Program>,

  #[serde(skip_serializing, skip_deserializing)]
  pub cl_device_kernel: Option<opencl3::kernel::Kernel>,

  #[serde(skip_serializing, skip_deserializing, default = "serde_empty_map_str_valtype")]
  pub cl_arg_types: HashMap<String, ValueType>,


}


fn serde_default_colmap() -> HashMap<String, String> { HashMap::<String, String>::new() }
fn serde_default_typemap() -> HashMap<String, ValueType> { HashMap::<String, ValueType>::new() }

//fn serde_default_data_columns_processed() -> Vec<RWColumn> { vec![] }
fn serde_default_data_constants() -> Vec<DataConstantValue> { vec![] }
fn serde_empty_map_str_valtype() -> HashMap<String, ValueType> { HashMap::<String, ValueType>::new() }



impl CL_Kernel {

  pub fn load_program(&mut self, cl_ctx: &opencl3::context::Context) -> Result<(), Box<dyn std::error::Error>>  {
    self.cl_device_program = Some(
      opencl3::program::Program::create_and_build_from_source(
        &cl_ctx,
        &self.source,
        &self.cl_program_compiler_options
      )?
    );
    if let Some(ref cl_device_program_ref) = self.cl_device_program {
      self.cl_device_kernel = Some(
        opencl3::kernel::Kernel::create(cl_device_program_ref, &self.name)?
      );
    }

    /*
    if let Some(ref cl_device_kernel_ref) = self.cl_device_kernel {
      // Read kernel argument type data & convert to intermediate formats

    }
    */

    Ok(())
  }

  /// transforms the loose data into CL buffers using the kernel's metadata. Order in the Vec<> corresponds to
  /// the order of data_columns_processed.
  pub fn data_to_cl_memory<T>(&self, data: utils::ListedData) -> Vec<opencl3::memory::Buffer<T>> {
    vec![]
  }
}



#[derive(Debug)]
pub enum CL_TaggedArgument {
  // These all correspond to PRIMITIVE* types
  Uint8Buffer  (opencl3::memory::Buffer<opencl3::types::cl_uchar>),
  Uint16Buffer (opencl3::memory::Buffer<opencl3::types::cl_ushort>),
  Uint32Buffer (opencl3::memory::Buffer<opencl3::types::cl_uint>),
  Uint64Buffer (opencl3::memory::Buffer<opencl3::types::cl_ulong>),

  Int8Buffer   (opencl3::memory::Buffer<opencl3::types::cl_char>),
  Int16Buffer  (opencl3::memory::Buffer<opencl3::types::cl_short>),
  Int32Buffer  (opencl3::memory::Buffer<opencl3::types::cl_int>),
  Int64Buffer  (opencl3::memory::Buffer<opencl3::types::cl_long>),

  FloatBuffer  (opencl3::memory::Buffer<opencl3::types::cl_float>),
  DoubleBuffer (opencl3::memory::Buffer<opencl3::types::cl_double>),

  // These all correspond to constant arguments
  Uint8  (opencl3::types::cl_uchar),
  Uint16 (opencl3::types::cl_ushort),
  Uint32 (opencl3::types::cl_uint),
  Uint64 (opencl3::types::cl_ulong),

  Int8   (opencl3::types::cl_char),
  Int16  (opencl3::types::cl_short),
  Int32  (opencl3::types::cl_int),
  Int64  (opencl3::types::cl_long),

  Float  (opencl3::types::cl_float),
  Double (opencl3::types::cl_double),
}

impl CL_TaggedArgument {
  pub fn from_value(v: &Value, cl_type_name_hint: &str) -> CL_TaggedArgument {
    match v {
      Value::Integer(int64_val) => {
        if cl_type_name_hint == "float" {
          CL_TaggedArgument::Float(*int64_val as f32)
        }
        else if cl_type_name_hint == "double" {
          CL_TaggedArgument::Double(*int64_val as f64)
        }
        else if cl_type_name_hint == "uchar" {
          CL_TaggedArgument::Uint8(*int64_val as u8)
        }
        else if cl_type_name_hint == "char" {
          CL_TaggedArgument::Int8(*int64_val as i8)
        }
        else if cl_type_name_hint == "ushort" {
          CL_TaggedArgument::Uint16(*int64_val as u16)
        }
        else if cl_type_name_hint == "short" {
          CL_TaggedArgument::Int16(*int64_val as i16)
        }
        else if cl_type_name_hint == "uint" {
          CL_TaggedArgument::Uint32(*int64_val as u32)
        }
        else if cl_type_name_hint == "int" {
          CL_TaggedArgument::Int32(*int64_val as i32)
        }
        else if cl_type_name_hint == "ulong" {
          CL_TaggedArgument::Uint64(*int64_val as u64)
        }
        else if cl_type_name_hint == "long" {
          CL_TaggedArgument::Int64(*int64_val as i64)
        }
        else {
          println!("[ Warning ] Unknown cl_type_name_hint={}, assuming long (aka i64)", cl_type_name_hint);
          CL_TaggedArgument::Int64(*int64_val)
        }
      },
      Value::Double(double_val) => {
        if cl_type_name_hint == "float" {
          CL_TaggedArgument::Float(*double_val as f32)
        }
        else if cl_type_name_hint == "double" {
          CL_TaggedArgument::Double(*double_val as f64)
        }
        else if cl_type_name_hint == "uchar" {
          CL_TaggedArgument::Uint8(*double_val as u8)
        }
        else if cl_type_name_hint == "char" {
          CL_TaggedArgument::Int8(*double_val as i8)
        }
        else if cl_type_name_hint == "ushort" {
          CL_TaggedArgument::Uint16(*double_val as u16)
        }
        else if cl_type_name_hint == "short" {
          CL_TaggedArgument::Int16(*double_val as i16)
        }
        else if cl_type_name_hint == "uint" {
          CL_TaggedArgument::Uint32(*double_val as u32)
        }
        else if cl_type_name_hint == "int" {
          CL_TaggedArgument::Int32(*double_val as i32)
        }
        else if cl_type_name_hint == "ulong" {
          CL_TaggedArgument::Uint64(*double_val as u64)
        }
        else if cl_type_name_hint == "long" {
          CL_TaggedArgument::Int64(*double_val as i64)
        }
        else {
          println!("[ Warning ] Unknown cl_type_name_hint={}, assuming double (aka f64)", cl_type_name_hint);
          CL_TaggedArgument::Double(*double_val)
        }
      },
      Value::String(string_val) => panic!("Cannot use a string value as a CL Kernel constant!"),
    }
  }


}

