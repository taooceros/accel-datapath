use clap::Parser;

mod cli;
mod custom_codec;
mod report;
mod runtime;
mod runtime_instrumentation;
mod service;
mod workload;

pub mod profile {
    tonic::include_proto!("tonicprofile");
}

use cli::{validate_args, Args};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

fn main() -> Result<(), BoxError> {
    let args = Args::parse();
    validate_args(&args)?;
    runtime::run(args)
}
