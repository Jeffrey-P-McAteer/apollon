

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




