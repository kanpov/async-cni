use std::path::Path;

use serde_json::{Map, Value};
use tokio::fs::read_to_string;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plugin {
    plugin_type: String,
    args: Option<Map<String, Value>>,
    capabilities: Option<Map<String, Value>>,
    plugin_options: Map<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginList {
    cni_version: String,
    cni_versions: Option<Vec<String>>,
    name: String,
    disable_check: bool,
    disable_gc: bool,
    plugins: Vec<Plugin>,
}

#[derive(Debug)]
pub enum CniDeserializationError {
    FileError(tokio::io::Error),
    SerdeError(serde_json::Error),
    RootIsNotObject,
    MissingKey,
    KeyOfWrongType,
    EmptyArray,
}

impl PluginList {
    pub async fn from_file(path: impl AsRef<Path>) -> Result<PluginList, CniDeserializationError> {
        let content = read_to_string(path)
            .await
            .map_err(|err| CniDeserializationError::FileError(err))?;
        Self::from_string(content)
    }

    pub fn from_string(content: impl AsRef<str>) -> Result<PluginList, CniDeserializationError> {
        let json_value: Value =
            serde_json::from_str(content.as_ref()).map_err(|err| CniDeserializationError::SerdeError(err))?;
        Self::from_json(&json_value)
    }

    pub fn from_json(json_value: &Value) -> Result<PluginList, CniDeserializationError> {
        let obj = json_value.as_object().ok_or(CniDeserializationError::RootIsNotObject)?;
        let cni_version = obj
            .get("cniVersion")
            .ok_or(CniDeserializationError::MissingKey)?
            .as_str()
            .ok_or(CniDeserializationError::KeyOfWrongType)?
            .to_string();
        let cni_versions = match json_value.get("cniVersions") {
            Some(list) => Some(
                list.as_array()
                    .ok_or(CniDeserializationError::KeyOfWrongType)?
                    .iter()
                    .map(|val| {
                        val.as_str()
                            .expect("CNI version inside list was not a string")
                            .to_string()
                    })
                    .collect::<Vec<_>>(),
            ),
            None => None,
        };
        if cni_versions.clone().is_some_and(|list| list.is_empty()) {
            return Err(CniDeserializationError::EmptyArray);
        }

        let name = obj
            .get("name")
            .ok_or(CniDeserializationError::MissingKey)?
            .as_str()
            .ok_or(CniDeserializationError::KeyOfWrongType)?
            .to_string();
        let disable_check = match obj.get("disableCheck") {
            Some(val) => val.as_bool().ok_or(CniDeserializationError::KeyOfWrongType)?,
            None => false,
        };
        let disable_gc = match obj.get("disableGC") {
            Some(val) => val.as_bool().ok_or(CniDeserializationError::KeyOfWrongType)?,
            None => false,
        };

        let plugin_json_array = obj
            .get("plugins")
            .ok_or(CniDeserializationError::MissingKey)?
            .as_array()
            .ok_or(CniDeserializationError::KeyOfWrongType)?;
        if plugin_json_array.is_empty() {
            return Err(CniDeserializationError::EmptyArray);
        }

        let mut plugins: Vec<Plugin> = Vec::with_capacity(plugin_json_array.len());
        for plugin_json_value in plugin_json_array {
            plugins.push(Plugin::from_json(plugin_json_value)?);
        }

        Ok(PluginList {
            cni_version,
            cni_versions,
            name,
            disable_check,
            disable_gc,
            plugins,
        })
    }
}

impl Plugin {
    pub fn from_json(json_value: &Value) -> Result<Plugin, CniDeserializationError> {
        let obj = json_value.as_object().ok_or(CniDeserializationError::RootIsNotObject)?;

        let mut plugin_type_option: Option<String> = None;
        let mut args: Option<Map<String, Value>> = None;
        let mut capabilities: Option<Map<String, Value>> = None;
        let mut plugin_options: Map<String, Value> = Map::new();

        for (key, value) in obj {
            match key.as_str() {
                "type" => {
                    plugin_type_option = Some(value.as_str().ok_or(CniDeserializationError::KeyOfWrongType)?.into());
                }
                "args" => args = Some(value.as_object().ok_or(CniDeserializationError::MissingKey)?.clone()),
                "capabilities" => {
                    capabilities = Some(value.as_object().ok_or(CniDeserializationError::MissingKey)?.clone())
                }
                _ => {
                    plugin_options.insert(key.clone(), value.clone());
                }
            }
        }

        let plugin_type = plugin_type_option.ok_or(CniDeserializationError::MissingKey)?;
        Ok(Plugin {
            plugin_type,
            args,
            capabilities,
            plugin_options,
        })
    }
}
