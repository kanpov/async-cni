use std::{collections::HashMap, net};

use invocation::{CniInvocation, CniInvocationArguments, CniInvocationError, CniInvocationOutput, CniInvocationTarget};
use plugins::CniPlugin;
use serde_json::Value;
use types::{CniAttachment, CniOperation};

pub mod invocation;
pub mod plugins;
pub mod types;

/// Perform a CNI invocation, moving in the invocation. This is the main function of tokio-cni.
pub async fn invoke<'a>(invocation: CniInvocation<'a>) -> Result<CniInvocationOutput, CniInvocationError> {
    match invocation.target {
        CniInvocationTarget::Plugin {
            plugin,
            cni_version,
            network_name,
        } => todo!(),
        CniInvocationTarget::PluginList(plugin_list) => {
            for plugin in &plugin_list.plugins {
                let location = invocation.locator.locate(&plugin.plugin_type).await;
                let location = match location {
                    Some(location) => location,
                    None => {
                        return Err(CniInvocationError::LocatorNotFound {
                            plugin: plugin.plugin_type.clone(),
                        })
                    }
                };

                invoke_single(
                    location.to_string_lossy().into_owned().as_str(),
                    plugin,
                    &invocation.target,
                    None,
                    &invocation.operation,
                    &invocation.arguments,
                )
                .await?;
            }
        }
    }

    todo!()
}

async fn invoke_single(
    program: &str,
    plugin: &CniPlugin,
    invocation_target: &CniInvocationTarget<'_>,
    previous_attachment: Option<&CniAttachment>,
    operation: &CniOperation,
    arguments: &CniInvocationArguments,
) -> Result<String, CniInvocationError> {
    let mut environment: HashMap<String, String> = HashMap::new();
    environment.insert("CNI_COMMAND".into(), operation.as_ref().into());

    if let Some(container_id) = &arguments.container_id {
        environment.insert("CNI_CONTAINERID".into(), container_id.as_ref().into());
    }

    if let Some(net_ns) = &arguments.net_ns {
        environment.insert("CNI_NETNS".into(), net_ns.into());
    }

    if let Some(interface_name) = &arguments.interface_name {
        environment.insert("CNI_IFNAME".into(), interface_name.as_ref().into());
    }

    if let Some(paths) = &arguments.paths {
        if !paths.is_empty() {
            let path_str = paths
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(":");
            environment.insert("CNI_PATH".into(), path_str);
        }
    }

    let stdin = derive_stdin(plugin, invocation_target, previous_attachment, arguments)?;
    dbg!(stdin);

    todo!()
}

fn derive_stdin(
    plugin: &CniPlugin,
    invocation_target: &CniInvocationTarget,
    previous_attachment: Option<&CniAttachment>,
    arguments: &CniInvocationArguments,
) -> Result<String, CniInvocationError> {
    // plugin options
    let mut map = plugin.plugin_options.clone();

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

    serde_json::to_string(&Value::Object(map)).map_err(CniInvocationError::JsonOperationFailed)
}
