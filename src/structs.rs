

#[derive(Debug, clap::Parser)]
pub struct Args {
    /// A data file (.csv, .json, etc.) containing T=0 data for the simulation.
    pub data_file_path: std::path::PathBuf,

    /// A data file (.csv, .json, etc.) containing entity & field deltas.
    /// One column in data_file_path and delta_file_path MUST be identical and is used to specify per-entity field delta functions.
    pub delta_file_path: std::path::PathBuf,

    /// Number of simulation steps to run
    #[arg(short, long, default_value_t = 1)]
    pub num_steps: u64,
}

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




