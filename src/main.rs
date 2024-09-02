
use clap::Parser;

pub mod structs;


fn main() {
  let args = structs::Args::parse();

  println!("args = {:?}", &args);



}
