
use std::collections::HashMap;


#[derive(Debug, clap::Parser)]
pub struct Args {
    /// A data file (.toml, .json, etc.) containing simulation configuration data.
    pub simcontrol_file_path: std::path::PathBuf,


    /// A data file (.csv, .json, etc.) containing T=0 data for the simulation.
    #[arg(short, long)]
    pub data_file_path: Option<std::path::PathBuf>,

    /// A data file (.toml) containing OpenCL kernels to be executed,
    /// and which is expected to supply the delta_file_path with functions to use.
    #[arg(short, long)]
    pub cl_kernels_file_path: Option<std::path::PathBuf>,

    /// Number of simulation steps to run
    #[arg(short, long)]
    pub num_steps: Option<u64>,

    /// Preferred GPU name to use. Pass "LIST" to list all GPUs detected on this system.
    #[arg(short, long)]
    pub preferred_gpu_name: Option<String>,

    /// Which attribute in delta_file_path holds the item's X position?
    #[arg(short, long)]
    pub gis_x_attr_name: Option<String>,

    /// Which attribute in delta_file_path holds the item's Y position?
    #[arg(short, long)]
    pub gis_y_attr_name: Option<String>,


}


#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SimControl_file { // utility to allow us to specify name of value
  pub simulation: SimControl,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SimControl {
    /// A data file (.csv, .json, etc.) containing T=0 data for the simulation.
    pub data_file_path: std::path::PathBuf,

    /// A data file (.toml) containing OpenCL kernels to be executed,
    /// and which is expected to supply the delta_file_path with functions to use.
    pub cl_kernels_file_path: std::path::PathBuf,

    /// Number of simulation steps to run
    #[serde(default = "serde_default_num_steps")]
    pub num_steps: u64,

    /// Preferred GPU name to use. Pass "LIST" to list all GPUs detected on this system.
    #[serde(default = "serde_empty_string")]
    pub preferred_gpu_name: String,

    #[serde(default = "serde_default_gis_x_attr_name")]
    pub gis_x_attr_name: String,
    #[serde(default = "serde_default_gis_y_attr_name")]
    pub gis_y_attr_name: String,

    #[serde(default = "serde_default_column_types")]
    pub column_types: HashMap<String, ValueType>,

}

fn serde_empty_string()              -> String { String::new() }
fn serde_default_num_steps()         -> u64    { 64 }
fn serde_default_gis_x_attr_name()   -> String { "X".to_string() }
fn serde_default_gis_y_attr_name()   -> String { "Y".to_string() }
fn serde_default_column_types()      -> HashMap<String, ValueType> { HashMap::<String, ValueType>::new() }











#[derive(Default, Debug, serde::Serialize)]
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












#[derive(Debug, serde::Serialize)]
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



#[derive(Debug, serde::Serialize)]
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







#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CL_Kernels {
  pub kernel: Vec<CL_Kernel>,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CL_Kernel {
  pub name: String,

  #[serde(default = "serde_default_column_types")]
  pub column_types: HashMap<String, ValueType>,

  pub data_columns_processed: Vec<RWColumn>,

  pub source: String,
}



