use std::collections::HashMap;

use crate::invocation::{
    CniInvocation, CniInvocationError, CniInvocationOverrides, CniInvocationResult, CniInvocationTarget, CniInvoker,
    CniLocator,
};
use crate::plugins::CniPlugin;
use crate::types::{CniAttachment, CniError, CniVersionObject};
use serde_json::Value;

/// Perform a CNI invocation, moving in the invocation. This is the main function of tokio-cni.
pub async fn invoke(
    invocation: CniInvocation,
    invocation_overrides: &CniInvocationOverrides,
    invocation_target: &CniInvocationTarget<'_>,
    invoker: &impl CniInvoker,
    locator: &impl CniLocator,
) -> Result<CniInvocationResult, CniInvocationError> {
    let mut invocation_output = CniInvocationResult {
        attachment: None,
        version_objects: HashMap::new(),
    };

    match invocation_target {
        CniInvocationTarget::Plugin {
            plugin,
            cni_version: _,
            name: _,
        } => {
            invoke_plugin(
                invocation,
                invocation_overrides,
                plugin,
                invocation_target,
                &mut invocation_output,
                invoker,
                locator,
            )
            .await?;
        }
        CniInvocationTarget::PluginList(plugin_list) => {
            let plugin_iter = match invocation {
                CniInvocation::Delete {
                    container_id: _,
                    net_ns: _,
                    interface_name: _,
                    attachment: _,
                    paths: _,
                } => plugin_list.plugins.iter().rev().collect::<Vec<_>>(),
                _ => plugin_list.plugins.iter().collect::<Vec<_>>(),
            };

            for plugin in plugin_iter {
                invoke_plugin(
                    invocation.clone(),
                    invocation_overrides,
                    plugin,
                    invocation_target,
                    &mut invocation_output,
                    invoker,
                    locator,
                )
                .await?;
            }
        }
    }

    Ok(invocation_output)
}

async fn invoke_plugin(
    invocation: CniInvocation,
    invocation_overrides: &CniInvocationOverrides,
    plugin: &CniPlugin,
    invocation_target: &CniInvocationTarget<'_>,
    invocation_output: &mut CniInvocationResult,
    invoker: &impl CniInvoker,
    locator: &impl CniLocator,
) -> Result<(), CniInvocationError> {
    let location = match locator.locate(&plugin.plugin_type).await {
        Some(location) => location,
        None => {
            return Err(CniInvocationError::PluginNotFoundByLocator);
        }
    };

    let mut environment: HashMap<String, String> = HashMap::new();
    environment.insert("CNI_COMMAND".into(), invocation.as_ref().into());

    let mut overrides: CniInvocationOverrides = invocation.into();
    overrides.merge_with(invocation_overrides);

    if let Some(container_id) = &overrides.container_id {
        environment.insert("CNI_CONTAINERID".into(), container_id.as_ref().into());
    }

    if let Some(net_ns) = &overrides.net_ns {
        environment.insert("CNI_NETNS".into(), net_ns.into());
    }

    if let Some(interface_name) = &overrides.interface_name {
        environment.insert("CNI_IFNAME".into(), interface_name.as_ref().into());
    }

    if let Some(paths) = &overrides.paths {
        if !paths.is_empty() {
            let path_str = paths
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(":");
            environment.insert("CNI_PATH".into(), path_str);
        }
    }

    let previous_attachment = overrides.attachment.as_ref().or(invocation_output.attachment.as_ref());
    let stdin = derive_stdin(plugin, &overrides, invocation_target, previous_attachment)?;
    let cni_output = invoker
        .invoke(&location, environment, stdin)
        .await
        .map_err(CniInvocationError::InvokerFailed)?;

    add_to_invocation_output(cni_output, plugin, invocation_output)?;

    Ok(())
}

fn add_to_invocation_output(
    cni_output: String,
    plugin: &CniPlugin,
    invocation_output: &mut CniInvocationResult,
) -> Result<(), CniInvocationError> {
    if let Ok(version_object) = serde_json::from_str::<CniVersionObject>(&cni_output) {
        invocation_output
            .version_objects
            .insert(plugin.plugin_type.clone(), version_object);
        return Ok(());
    }

    if let Ok(attachment) = serde_json::from_str::<CniAttachment>(&cni_output) {
        invocation_output.attachment = Some(attachment);
        return Ok(());
    }

    if let Ok(error) = serde_json::from_str::<CniError>(&cni_output) {
        return Err(CniInvocationError::PluginProducedError(error));
    }

    if cni_output.trim().is_empty() {
        return Ok(());
    }

    Err(CniInvocationError::PluginProducedUnrecognizableOutput(cni_output))
}

fn derive_stdin(
    plugin: &CniPlugin,
    invocation_overrides: &CniInvocationOverrides,
    invocation_target: &CniInvocationTarget,
    previous_attachment: Option<&CniAttachment>,
) -> Result<String, CniInvocationError> {
    // plugin options
    let mut map = plugin.plugin_options.clone();

    // type
    map.insert("type".into(), Value::String(plugin.plugin_type.clone()));

    // name
    let network_name: String = match invocation_target {
        CniInvocationTarget::Plugin {
            plugin: _,
            cni_version: _,
            name,
        } => name.as_ref().to_owned(),
        CniInvocationTarget::PluginList(plugin_list) => plugin_list.name.as_ref().to_owned(),
    };
    map.insert("name".into(), Value::String(network_name));

    // cni version with override possibility
    let mut cni_version = match invocation_target {
        CniInvocationTarget::Plugin {
            plugin: _,
            cni_version,
            name: _,
        } => cni_version.clone(),
        CniInvocationTarget::PluginList(plugin_list) => plugin_list.cni_version.clone(),
    };
    if let Some(new_cni_version) = &invocation_overrides.cni_version {
        cni_version = new_cni_version.clone();
    }
    map.insert("cniVersion".into(), Value::String(cni_version));

    // capabilities as runtimeConfig
    if let Some(capabilities) = &plugin.capabilities {
        map.insert("runtimeConfig".into(), Value::Object(capabilities.clone()));
    }

    // args
    if let Some(args) = &plugin.args {
        map.insert("args".into(), Value::Object(args.clone()));
    }

    // previous attachment as prevResult
    if let Some(attachment) = previous_attachment {
        let attachment_value = serde_json::to_value(attachment).map_err(CniInvocationError::JsonOperationFailed)?;
        map.insert("prevResult".into(), attachment_value);
    }

    // gc valid attachments
    if let Some(valid_attachments) = &invocation_overrides.valid_attachments {
        let mut vec: Vec<Value> = Vec::with_capacity(valid_attachments.len());

        for valid_attachment in valid_attachments {
            vec.push(serde_json::to_value(valid_attachment).map_err(CniInvocationError::JsonOperationFailed)?);
        }

        map.insert("cni.dev/valid-attachments".into(), Value::Array(vec));
    }

    serde_json::to_string(&Value::Object(map)).map_err(CniInvocationError::JsonOperationFailed)
}

impl AsRef<str> for CniInvocation {
    fn as_ref(&self) -> &str {
        match self {
            CniInvocation::Add {
                container_id: _,
                net_ns: _,
                interface_name: _,
                paths: _,
            } => "ADD",
            CniInvocation::Delete {
                container_id: _,
                net_ns: _,
                interface_name: _,
                attachment: _,
                paths: _,
            } => "DEL",
            CniInvocation::Check {
                container_id: _,
                net_ns: _,
                interface_name: _,
                attachment: _,
            } => "CHECK",
            CniInvocation::Status => "STATUS",
            CniInvocation::Version => "VERSION",
            CniInvocation::GarbageCollect {
                paths: _,
                valid_attachments: _,
            } => "GC",
        }
    }
}

impl From<CniInvocation> for CniInvocationOverrides {
    fn from(value: CniInvocation) -> Self {
        let mut builder = CniInvocationOverrides::new();

        match value {
            CniInvocation::Add {
                container_id,
                net_ns,
                interface_name,
                paths,
            } => {
                builder
                    .container_id(container_id)
                    .net_ns(net_ns)
                    .interface_name(interface_name)
                    .paths(paths);
            }
            CniInvocation::Delete {
                container_id,
                net_ns,
                interface_name,
                attachment,
                paths,
            } => {
                builder
                    .container_id(container_id)
                    .net_ns(net_ns)
                    .interface_name(interface_name)
                    .attachment(attachment)
                    .paths(paths);
            }
            CniInvocation::Check {
                container_id,
                net_ns,
                interface_name,
                attachment,
            } => {
                builder
                    .container_id(container_id)
                    .net_ns(net_ns)
                    .interface_name(interface_name)
                    .attachment(attachment);
            }
            CniInvocation::GarbageCollect {
                paths,
                valid_attachments,
            } => {
                builder.paths(paths).valid_attachments(valid_attachments);
            }
            _ => {}
        }

        builder
    }
}

impl CniInvocationOverrides {
    fn merge_with(&mut self, other: &CniInvocationOverrides) {
        if let Some(container_id) = &other.container_id {
            self.container_id = Some(container_id.clone());
        }

        if let Some(net_ns) = &other.net_ns {
            self.net_ns = Some(net_ns.clone());
        }

        if let Some(interface_name) = &other.interface_name {
            self.interface_name = Some(interface_name.clone());
        }

        if let Some(paths) = &other.paths {
            self.paths = Some(paths.clone());
        }

        if let Some(attachment) = &other.attachment {
            self.attachment = Some(attachment.clone());
        }

        if let Some(valid_attachments) = &other.valid_attachments {
            self.valid_attachments = Some(valid_attachments.clone());
        }

        if let Some(cni_version) = &other.cni_version {
            self.cni_version = Some(cni_version.clone());
        }
    }
}
