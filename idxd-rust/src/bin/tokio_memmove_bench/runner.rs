use crate::artifact::BenchmarkArtifact;
use crate::cli::{Backend, CliArgs};
use crate::hardware::hardware_artifact;
use crate::software::software_artifact;

pub(crate) async fn execute(args: &CliArgs) -> BenchmarkArtifact {
    match args.backend {
        Backend::Software => software_artifact(args).await,
        Backend::Hardware => hardware_artifact(args).await,
    }
}
