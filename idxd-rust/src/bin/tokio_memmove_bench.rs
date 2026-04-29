use std::env;
use std::io::{self, Write};
use std::process::ExitCode;

#[path = "tokio_memmove_bench/artifact.rs"]
mod artifact;
#[path = "tokio_memmove_bench/cli.rs"]
mod cli;
#[path = "tokio_memmove_bench/failure.rs"]
mod failure;
#[path = "tokio_memmove_bench/hardware.rs"]
mod hardware;
#[path = "tokio_memmove_bench/modes.rs"]
mod modes;
#[path = "tokio_memmove_bench/runner.rs"]
mod runner;
#[path = "tokio_memmove_bench/software.rs"]
mod software;

use artifact::emit_artifact;
use cli::{CliArgs, ParseOutcome, print_help};
use runner::execute;

#[tokio::main(flavor = "current_thread")]
async fn main() -> ExitCode {
    match CliArgs::parse(env::args().skip(1)) {
        Ok(ParseOutcome::Help) => {
            print_help();
            ExitCode::SUCCESS
        }
        Ok(ParseOutcome::Run(args)) => match run(args).await {
            Ok(exit) => exit,
            Err(err) => {
                let _ = writeln!(io::stderr(), "tokio_memmove_bench: {err}");
                ExitCode::from(2)
            }
        },
        Err(err) => {
            let _ = writeln!(io::stderr(), "tokio_memmove_bench: {err}");
            ExitCode::from(2)
        }
    }
}

async fn run(args: CliArgs) -> Result<ExitCode, String> {
    let artifact = execute(&args).await;
    emit_artifact(&args, &artifact)?;
    Ok(if artifact.ok {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    })
}
