use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Stdio,
};

use async_trait::async_trait;
use tokio::{
    io::{self, AsyncWriteExt},
    process::Command,
};

use crate::{
    plugins::{CniPlugin, CniPluginList},
    types::{CniAttachment, CniContainerId, CniError, CniInterfaceName, CniOperation, CniVersionObject},
};

pub type CniResult = Result<CniInvocationOutput, CniInvocationError>;

pub struct CniInvocationOutput {
    pub attachment: Option<CniAttachment>,
    pub version_objects: Vec<CniVersionObject>,
}

pub struct CniInvocationError {
    pub error: CniError,
    pub partial_output: CniInvocationOutput,
    pub cause_plugin: String,
    pub cause_plugin_list: Option<String>,
}

pub struct CniInvocation<'a> {
    pub operation: CniOperation,
    pub arguments: CniInvocationArguments,
    pub target: CniInvocationTarget<'a>,
    pub invoker: Box<dyn CniInvoker>,
    pub locator: Box<dyn CniLocator>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CniInvocationTarget<'a> {
    Plugin(&'a CniPlugin),
    PluginList(&'a CniPluginList),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CniInvocationArguments {
    pub container_id: Option<CniContainerId>,
    pub net_ns: Option<String>,
    pub interface_name: Option<CniInterfaceName>,
    pub paths: Option<Vec<PathBuf>>,
}

#[async_trait]
pub trait CniLocator {
    async fn locate(&self, plugin_type: &str) -> Option<PathBuf>;
}

pub struct MappedCniLocator {
    pub lookup_map: HashMap<String, PathBuf>,
}

#[async_trait]
impl CniLocator for MappedCniLocator {
    async fn locate(&self, plugin_type: &str) -> Option<PathBuf> {
        self.lookup_map.get(plugin_type).map(|path_buf| path_buf.clone())
    }
}

pub struct DirectoryCniLocator {
    pub directory_path: PathBuf,
    pub exact_name: bool,
}

#[async_trait]
impl CniLocator for DirectoryCniLocator {
    async fn locate(&self, plugin_type: &str) -> Option<PathBuf> {
        let mut read_dir = tokio::fs::read_dir(&self.directory_path).await.ok()?;

        while let Some(entry) = read_dir.next_entry().await.ok()? {
            if entry.file_name() == plugin_type
                || (!self.exact_name && entry.file_name().to_string_lossy().contains(plugin_type))
            {
                return Some(entry.path());
            }
        }

        None
    }
}

#[async_trait]
pub trait CniInvoker {
    async fn invoke(
        &self,
        program: &Path,
        environment: HashMap<String, String>,
        stdin: String,
    ) -> Result<String, io::Error>;
}

pub struct RootfulCniInvoker {}

#[async_trait]
impl CniInvoker for RootfulCniInvoker {
    async fn invoke(
        &self,
        program: &Path,
        environment: HashMap<String, String>,
        stdin: String,
    ) -> Result<String, io::Error> {
        let mut command = Command::new(program);
        command
            .envs(environment)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn()?;
        let mut child_stdin = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("Stdin not found despite having been piped"))?;
        child_stdin.write_all(stdin.as_bytes()).await?;
        child_stdin.flush().await?;
        drop(child_stdin); // EOF

        let output = child.wait_with_output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        if stdout.len() > stderr.len() {
            Ok(stdout.into())
        } else {
            Ok(stderr.into())
        }
    }
}

pub struct SuCniInvoker {
    pub su_path: PathBuf,
    pub password: String,
}

#[async_trait]
impl CniInvoker for SuCniInvoker {
    async fn invoke(
        &self,
        program: &Path,
        environment: HashMap<String, String>,
        stdin: String,
    ) -> Result<String, io::Error> {
        let mut command = Command::new(self.su_path.as_os_str());
        command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let mut child = command.spawn()?;
        let mut child_stdin = child.stdin.take().ok_or_else(|| io::Error::other("Stdin not found"))?;
        child_stdin.write_all((self.password.clone() + "\n").as_bytes()).await?;

        let full_command = build_env_string(environment) + program.to_string_lossy().to_string().as_str() + " ; exit\n";
        child_stdin.write_all(full_command.as_bytes()).await?;
        child_stdin.write_all(stdin.as_bytes()).await?;
        drop(child_stdin); // EOF

        let output = child.wait_with_output().await?;
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
        if stderr.contains("fail") {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Authentication was forbidden",
            ));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}

pub struct SudoCniInvoker {
    pub sudo_path: PathBuf,
    pub password: Option<String>,
}

#[async_trait]
impl CniInvoker for SudoCniInvoker {
    async fn invoke(
        &self,
        program: &Path,
        environment: HashMap<String, String>,
        stdin: String,
    ) -> Result<String, io::Error> {
        let full_command = build_env_string(environment) + program.to_string_lossy().to_string().as_str();
        let mut command = Command::new(self.sudo_path.as_os_str());
        command
            .stdout(Stdio::piped())
            .stdin(Stdio::piped())
            .stderr(Stdio::piped())
            .arg("-S")
            .arg(full_command);
        let mut child = command.spawn()?;
        let mut child_stdin = child.stdin.take().ok_or_else(|| io::Error::other("Stdin not found"))?;

        if let Some(password) = &self.password {
            child_stdin.write_all((password.to_string() + "\n").as_bytes()).await?;
        }

        child_stdin.write_all(stdin.as_bytes()).await?;
        drop(child_stdin); // EOF

        let output = child.wait_with_output().await?;
        let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
        let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

        if stderr.contains("Sorry, try again") {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Sudo rejected the given password",
            ));
        }

        Ok(stdout)
    }
}

fn build_env_string(environment: HashMap<String, String>) -> String {
    let mut env_string = String::new();
    for (key, value) in environment {
        env_string.push_str(&key);
        env_string.push('=');
        env_string.push_str(&value);
        env_string.push(' ');
    }
    env_string
}
