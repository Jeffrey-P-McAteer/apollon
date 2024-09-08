

#[derive(Debug, clap::Parser)]
pub struct Args {
    /// A data file (.toml, .json, etc.) containing simulation configuration data.
    pub simcontrol_file_path: std::path::PathBuf,


    /// A data file (.csv, .json, etc.) containing T=0 data for the simulation.
    #[arg(short, long)]
    pub data_file_path: Option<std::path::PathBuf>,

    /// A data file (.csv, .json, etc.) containing entity & field deltas.
    /// One column in data_file_path and delta_file_path MUST be identical and is used to specify per-entity field delta functions.
    #[arg(short, long)]
    pub delta_file_path: Option<std::path::PathBuf>,

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

    /// A data file (.csv, .json, etc.) containing entity & field deltas.
    /// One column in data_file_path and delta_file_path MUST be identical and is used to specify per-entity field delta functions.
    pub delta_file_path: std::path::PathBuf,

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

}

fn serde_empty_string()              -> String { String::new() }
fn serde_default_num_steps()         -> u64    { 64 }
fn serde_default_gis_x_attr_name()   -> String { "X".to_string() }
fn serde_default_gis_y_attr_name()   -> String { "Y".to_string() }



//#[derive(Debug, serde::Serialize, serde::Deserialize)]
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



#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CL_Kernels {
  pub kernel: Vec<CL_Kernel>,
}

#[derive(Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CL_Kernel {
  pub name: String,
  pub source: String,
}
