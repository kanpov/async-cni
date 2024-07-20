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
    types::{
        CniAttachment, CniContainerId, CniError, CniInterfaceName, CniName, CniValidAttachment, CniVersion,
        CniVersionObject,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CniInvocationResult {
    pub attachment: Option<CniAttachment>,
    pub version_objects: HashMap<String, CniVersionObject>,
}

#[derive(Debug)]
pub enum CniInvocationError {
    PluginNotFoundByLocator,
    InvokerFailed(io::Error),
    JsonOperationFailed(serde_json::Error),
    PluginProducedUnrecognizableOutput(String),
    PluginProducedError(CniError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CniInvocationArguments {
    pub(crate) container_id: Option<CniContainerId>,
    pub(crate) net_ns: Option<String>,
    pub(crate) interface_name: Option<CniInterfaceName>,
    pub(crate) paths: Option<Vec<PathBuf>>,
    pub(crate) attachment: Option<CniAttachment>,
    pub(crate) valid_attachments: Option<Vec<CniValidAttachment>>,
    pub(crate) cni_version: Option<CniVersion>,
}

impl CniInvocationArguments {
    pub fn new() -> Self {
        Self {
            container_id: None,
            net_ns: None,
            interface_name: None,
            paths: None,
            attachment: None,
            valid_attachments: None,
            cni_version: None,
        }
    }

    pub fn container_id(&mut self, container_id: CniContainerId) -> &mut Self {
        self.container_id = Some(container_id);
        self
    }

    pub fn net_ns(&mut self, net_ns: impl Into<String>) -> &mut Self {
        self.net_ns = Some(net_ns.into());
        self
    }

    pub fn interface_name(&mut self, interface_name: CniInterfaceName) -> &mut Self {
        self.interface_name = Some(interface_name);
        self
    }

    pub fn paths<P: Into<PathBuf>>(&mut self, paths: Vec<P>) -> &mut Self {
        self.paths = Some(paths.into_iter().map(|p| p.into()).collect());
        self
    }

    pub fn attachment(&mut self, attachment: CniAttachment) -> &mut Self {
        self.attachment = Some(attachment);
        self
    }

    pub fn valid_attachments(&mut self, valid_attachments: Vec<CniValidAttachment>) -> &mut Self {
        self.valid_attachments = Some(valid_attachments);
        self
    }

    pub fn cni_version(&mut self, cni_version: CniVersion) -> &mut Self {
        self.cni_version = Some(cni_version);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CniInvocationTarget<'a> {
    Plugin {
        plugin: &'a CniPlugin,
        name: CniName,
        cni_version: CniVersion,
    },
    PluginList(&'a CniPluginList),
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
