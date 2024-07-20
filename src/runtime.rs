use std::collections::HashMap;

use crate::invocation::{
    CniInvocationArguments, CniInvocationError, CniInvocationResult, CniInvocationTarget, CniInvoker, CniLocator,
};
use crate::plugins::CniPlugin;
use crate::types::{CniAttachment, CniError, CniOperation, CniVersionObject};
use serde_json::Value;

/// Perform a CNI invocation. This is the main function of tokio-cni.
pub async fn invoke(
    operation: CniOperation,
    invocation_arguments: &CniInvocationArguments,
    invocation_target: &CniInvocationTarget<'_>,
    invoker: &impl CniInvoker,
    locator: &impl CniLocator,
) -> Result<CniInvocationResult, CniInvocationError> {
    let mut invocation_result = CniInvocationResult {
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
                operation,
                invocation_arguments,
                plugin,
                invocation_target,
                &mut invocation_result,
                invoker,
                locator,
            )
            .await?;
        }
        CniInvocationTarget::PluginList(plugin_list) => {
            let plugin_iter = match operation {
                CniOperation::Delete => plugin_list.plugins.iter().rev().collect::<Vec<_>>(),
                _ => plugin_list.plugins.iter().collect::<Vec<_>>(),
            };

            for plugin in plugin_iter {
                invoke_plugin(
                    operation,
                    invocation_arguments,
                    plugin,
                    invocation_target,
                    &mut invocation_result,
                    invoker,
                    locator,
                )
                .await?;
            }
        }
    }

    Ok(invocation_result)
}

async fn invoke_plugin(
    operation: CniOperation,
    invocation_arguments: &CniInvocationArguments,
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
    environment.insert(
        "CNI_COMMAND".into(),
        match operation {
            CniOperation::Add => "ADD".into(),
            CniOperation::Delete => "DEL".into(),
            CniOperation::Check => "CHECK".into(),
            CniOperation::Version => "VERSION".into(),
            CniOperation::Status => "STATUS".into(),
            CniOperation::GarbageCollect => "GC".into(),
        },
    );

    if let Some(container_id) = &invocation_arguments.container_id {
        environment.insert("CNI_CONTAINERID".into(), container_id.as_ref().into());
    }

    if let Some(net_ns) = &invocation_arguments.network_namespace {
        environment.insert("CNI_NETNS".into(), net_ns.into());
    }

    if let Some(interface_name) = &invocation_arguments.interface_name {
        environment.insert("CNI_IFNAME".into(), interface_name.as_ref().into());
    }

    if let Some(paths) = &invocation_arguments.paths {
        if !paths.is_empty() {
            let path_str = paths
                .iter()
                .map(|p| p.to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join(":");
            environment.insert("CNI_PATH".into(), path_str);
        }
    }

    let previous_attachment = invocation_arguments
        .attachment
        .as_ref()
        .or(invocation_output.attachment.as_ref());
    let stdin = derive_stdin(plugin, &invocation_arguments, invocation_target, previous_attachment)?;
    let cni_output = invoker
        .invoke(&location, environment, stdin)
        .await
        .map_err(CniInvocationError::InvokerFailed)?;

    add_to_invocation_result(cni_output, plugin, invocation_output)?;

    Ok(())
}

fn add_to_invocation_result(
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
    arguments: &CniInvocationArguments,
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
    if let Some(new_cni_version) = &arguments.cni_version {
        cni_version = new_cni_version.clone();
    }
    map.insert("cniVersion".into(), Value::String(cni_version.as_ref().to_owned()));

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
    if let Some(valid_attachments) = &arguments.valid_attachments {
        let mut vec: Vec<Value> = Vec::with_capacity(valid_attachments.len());

        for valid_attachment in valid_attachments {
            vec.push(serde_json::to_value(valid_attachment).map_err(CniInvocationError::JsonOperationFailed)?);
        }

        map.insert("cni.dev/valid-attachments".into(), Value::Array(vec));
    }

    serde_json::to_string(&Value::Object(map)).map_err(CniInvocationError::JsonOperationFailed)
}
