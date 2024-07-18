use std::{
    collections::HashMap,
    future::Future,
    path::{Path, PathBuf},
    pin::Pin,
    process::Stdio,
};

use tokio::{io::AsyncWriteExt, process::Command};

pub struct InvocationOptions {}

pub trait PluginLocator {
    fn locate(plugin_type: &str) -> Pin<Box<dyn Future<Output = Option<PathBuf>> + Send>>;
}

pub trait ProcessExecutor {
    type Error;

    fn execute(
        program: &Path,
        environment: HashMap<String, String>,
        stdin: String,
    ) -> Pin<Box<dyn Future<Output = Result<std::process::Output, Self::Error>> + Send + '_>>;
}

#[cfg(feature = "tokio-executor")]
pub struct TokioProcessExecutor {}

#[cfg(feature = "tokio-executor")]
impl ProcessExecutor for TokioProcessExecutor {
    type Error = tokio::io::Error;

    fn execute(
        program: &Path,
        environment: HashMap<String, String>,
        stdin: String,
    ) -> Pin<Box<dyn Future<Output = Result<std::process::Output, Self::Error>> + Send + '_>> {
        Box::pin(async move {
            let mut command = Command::new(program);
            command
                .envs(environment)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .stdin(Stdio::piped());
            let mut child = command.spawn()?;
            let mut stdin_writer = child
                .stdin
                .take()
                .ok_or(tokio::io::Error::other("Stdin not piped correctly"))?;
            stdin_writer.write_all(stdin.as_bytes()).await?;
            drop(stdin_writer);
            command.output().await
        })
    }
}
