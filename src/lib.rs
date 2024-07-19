use std::collections::HashMap;

use invocation::{CniInvocation, CniInvocationArguments, CniInvocationError, CniInvocationOutput, CniInvocationTarget};
use plugins::CniPlugin;
use serde_json::Value;
use types::{CniAttachment, CniError, CniOperation, CniVersionObject};

pub mod invocation;
pub mod plugins;
pub mod types;

/// Perform a CNI invocation, moving in the invocation. This is the main function of tokio-cni.
pub async fn invoke<'a>(invocation: CniInvocation<'a>) -> Result<CniInvocationOutput, CniInvocationError> {
    let mut invocation_output = CniInvocationOutput {
        attachment: None,
        version_objects: Vec::new(),
    };

    match invocation.target {
        CniInvocationTarget::Plugin {
            plugin,
            cni_version: _,
            network_name: _,
        } => {
            invoke_plugin(&invocation, plugin, &mut invocation_output).await?;
        }
        CniInvocationTarget::PluginList(plugin_list) => {
            let plugin_iter = match invocation.operation {
                CniOperation::Delete => plugin_list.plugins.iter().rev().collect::<Vec<_>>(),
                _ => plugin_list.plugins.iter().collect::<Vec<_>>(),
            };

            for plugin in plugin_iter {
                invoke_plugin(&invocation, plugin, &mut invocation_output).await?;
            }
        }
    }

    Ok(invocation_output)
}

async fn invoke_plugin(
    invocation: &CniInvocation<'_>,
    plugin: &CniPlugin,
    invocation_output: &mut CniInvocationOutput,
) -> Result<(), CniInvocationError> {
    let location = match invocation.locator.locate(&plugin.plugin_type).await {
        Some(location) => location,
        None => {
            return Err(CniInvocationError::PluginNotFoundByLocator);
        }
    };

    let mut environment: HashMap<String, String> = HashMap::new();
    environment.insert("CNI_COMMAND".into(), invocation.operation.as_ref().into());

    if let Some(container_id) = &invocation.arguments.container_id {
        environment.insert("CNI_CONTAINERID".into(), container_id.as_ref().into());
    }

    if let Some(net_ns) = &invocation.arguments.net_ns {
        environment.insert("CNI_NETNS".into(), net_ns.into());
    }

    if let Some(interface_name) = &invocation.arguments.interface_name {
        environment.insert("CNI_IFNAME".into(), interface_name.as_ref().into());
    }

    if let Some(paths) = &invocation.arguments.paths {
        if !paths.is_empty() {
            let path_str = paths
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(":");
            environment.insert("CNI_PATH".into(), path_str);
        }
    }

    let stdin = derive_stdin(
        plugin,
        &invocation.target,
        invocation_output.attachment.as_ref(),
        &invocation.arguments,
    )?;
    let cni_output = invocation
        .invoker
        .invoke(&location, environment, stdin)
        .await
        .map_err(CniInvocationError::InvokerFailed)?;

    add_to_invocation_output(cni_output, invocation_output)?;

    Ok(())
}

fn add_to_invocation_output(
    cni_output: String,
    invocation_output: &mut CniInvocationOutput,
) -> Result<(), CniInvocationError> {
    if let Ok(attachment) = serde_json::from_str::<CniAttachment>(&cni_output) {
        invocation_output.attachment = Some(attachment);
        return Ok(());
    }

    if let Ok(version_object) = serde_json::from_str::<CniVersionObject>(&cni_output) {
        invocation_output.version_objects.push(version_object);
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
    invocation_target: &CniInvocationTarget,
    previous_attachment: Option<&CniAttachment>,
    arguments: &CniInvocationArguments,
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
            network_name,
        } => network_name.clone().into(),
        CniInvocationTarget::PluginList(plugin_list) => plugin_list.name.clone(),
    };
    map.insert("name".into(), Value::String(network_name));

    // cni version with override possibility
    let mut cni_version = match invocation_target {
        CniInvocationTarget::Plugin {
            plugin: _,
            cni_version,
            network_name: _,
        } => cni_version.clone(),
        CniInvocationTarget::PluginList(plugin_list) => plugin_list.cni_version.clone(),
    };
    if let Some(new_cni_version) = &arguments.overridden_cni_version {
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

    // previous attachment (optionally) as prevResult
    if let Some(attachment) = previous_attachment {
        let attachment_value = serde_json::to_value(attachment).map_err(CniInvocationError::JsonOperationFailed)?;
        map.insert("prevResult".into(), attachment_value);
    }

    serde_json::to_string_pretty(&Value::Object(map)).map_err(CniInvocationError::JsonOperationFailed)
}
