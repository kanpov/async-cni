use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Output,
};

use async_trait::async_trait;

pub struct InvocationOptions {}

#[async_trait]
pub trait CniPluginLocator {
    async fn locate(plugin_type: &str) -> Option<PathBuf>;
}

#[async_trait]
pub trait CniInvoker {
    type Error;

    async fn invoke(program: &Path, environment: HashMap<String, String>, stdin: String)
        -> Result<Output, Self::Error>;
}
