use std::{collections::VecDeque, path::Path};

use async_trait::async_trait;
use serde_json::{Map, Value};
use tokio::{
    fs::{read_to_string, write},
    io,
};

use crate::types::{CniName, CniValidationError, CniVersion};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CniPluginList {
    pub cni_version: CniVersion,
    pub cni_versions: Option<Vec<CniVersion>>,
    pub name: CniName,
    pub disable_check: bool,
    pub disable_gc: bool,
    pub plugins: Vec<CniPlugin>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CniPlugin {
    pub plugin_type: String,
    pub args: Option<Map<String, Value>>,
    pub capabilities: Option<Map<String, Value>>,
    pub plugin_options: Map<String, Value>,
}

#[derive(Debug)]
pub enum CniDeserializationError {
    FileError(io::Error),
    SerdeError(serde_json::Error),
    RootIsNotObject,
    MissingKey,
    KeyOfWrongType,
    EmptyArray,
    MalformedName(CniValidationError),
    MalformedVersion(CniValidationError),
}

#[derive(Debug)]
pub enum CniSerializationError {
    FileError(io::Error),
    SerdeError(serde_json::Error),
    OverlappingKey,
}

#[async_trait]
pub trait CniDeserializable: Sized {
    async fn from_file(path: impl AsRef<Path> + Send) -> Result<Self, CniDeserializationError> {
        let content = read_to_string(path).await.map_err(CniDeserializationError::FileError)?;
        Self::from_string(content)
    }

    fn from_string(content: impl AsRef<str>) -> Result<Self, CniDeserializationError> {
        let json_value: Value =
            serde_json::from_str(content.as_ref()).map_err(|err| CniDeserializationError::SerdeError(err))?;
        Self::from_json_value(json_value)
    }

    fn from_json_value(json_value: Value) -> Result<Self, CniDeserializationError>;
}

#[async_trait]
pub trait CniSerializable: Sized {
    async fn to_file(self, path: impl AsRef<Path> + Send) -> Result<(), CniSerializationError> {
        let content = self.to_string()?;
        write(path, content).await.map_err(CniSerializationError::FileError)
    }

    fn to_string(self) -> Result<String, CniSerializationError> {
        let json_value = self.to_json_value()?;
        serde_json::to_string(&json_value).map_err(|err| CniSerializationError::SerdeError(err))
    }

    fn to_json_value(self) -> Result<Value, CniSerializationError>;
}

impl CniDeserializable for CniPluginList {
    fn from_json_value(mut json_value: Value) -> Result<Self, CniDeserializationError> {
        let obj = json_value
            .as_object_mut()
            .ok_or(CniDeserializationError::RootIsNotObject)?;
        let cni_version: CniVersion = CniVersion::parse(
            obj.remove("cniVersion")
                .ok_or(CniDeserializationError::MissingKey)?
                .as_str()
                .ok_or(CniDeserializationError::KeyOfWrongType)?,
        )
        .map_err(CniDeserializationError::MalformedVersion)?;
        let cni_versions = match obj.remove("cniVersions") {
            Some(list) => {
                let list = list.as_array().ok_or(CniDeserializationError::KeyOfWrongType)?;
                let mut parsed_list: Vec<CniVersion> = Vec::with_capacity(list.len());
                for list_value in list {
                    parsed_list.push(
                        CniVersion::parse(list_value.as_str().ok_or(CniDeserializationError::KeyOfWrongType)?)
                            .map_err(CniDeserializationError::MalformedVersion)?,
                    );
                }
                if parsed_list.is_empty() {
                    return Err(CniDeserializationError::EmptyArray);
                }
                Some(parsed_list)
            }
            None => None,
        };

        let name = CniName::new(
            obj.remove("name")
                .ok_or(CniDeserializationError::MissingKey)?
                .as_str()
                .ok_or(CniDeserializationError::KeyOfWrongType)?,
        )
        .map_err(CniDeserializationError::MalformedName)?;

        let disable_check = match obj.remove("disableCheck") {
            Some(val) => val.as_bool().ok_or(CniDeserializationError::KeyOfWrongType)?,
            None => false,
        };
        let disable_gc = match obj.remove("disableGC") {
            Some(val) => val.as_bool().ok_or(CniDeserializationError::KeyOfWrongType)?,
            None => false,
        };

        let plugin_jsons = obj.remove("plugins").ok_or(CniDeserializationError::MissingKey)?;
        let mut plugin_jsons = match plugin_jsons {
            Value::Array(array) => VecDeque::from(array),
            _ => return Err(CniDeserializationError::KeyOfWrongType),
        };
        if plugin_jsons.is_empty() {
            return Err(CniDeserializationError::EmptyArray);
        }

        let mut plugins: Vec<CniPlugin> = Vec::with_capacity(plugin_jsons.len());
        while let Some(plugin_json) = plugin_jsons.pop_front() {
            plugins.push(CniPlugin::from_json_value(plugin_json)?);
        }

        Ok(CniPluginList {
            cni_version,
            cni_versions,
            name,
            disable_check,
            disable_gc,
            plugins,
        })
    }
}

impl CniDeserializable for CniPlugin {
    fn from_json_value(json_value: Value) -> Result<Self, CniDeserializationError> {
        let obj = match json_value {
            Value::Object(x) => x,
            _ => return Err(CniDeserializationError::KeyOfWrongType),
        };

        let mut plugin_type_option: Option<String> = None;
        let mut args: Option<Map<String, Value>> = None;
        let mut capabilities: Option<Map<String, Value>> = None;
        let mut plugin_options: Map<String, Value> = Map::new();

        for (key, value) in obj.into_iter() {
            match key.as_str() {
                "type" => {
                    plugin_type_option = Some(value.as_str().ok_or(CniDeserializationError::KeyOfWrongType)?.into());
                }
                "args" => {
                    args = Some(match value {
                        Value::Object(x) => x,
                        _ => return Err(CniDeserializationError::KeyOfWrongType),
                    })
                }
                "capabilities" => {
                    capabilities = Some(match value {
                        Value::Object(x) => x,
                        _ => return Err(CniDeserializationError::KeyOfWrongType),
                    });
                }
                _ => {
                    plugin_options.insert(key, value);
                }
            }
        }

        let plugin_type = plugin_type_option.ok_or(CniDeserializationError::MissingKey)?;
        Ok(CniPlugin {
            plugin_type,
            args,
            capabilities,
            plugin_options,
        })
    }
}

impl CniSerializable for CniPluginList {
    fn to_json_value(self) -> Result<Value, CniSerializationError> {
        let mut map = Map::new();

        map.insert("cniVersion".into(), Value::String(self.cni_version.into()));
        if let Some(cni_versions) = self.cni_versions {
            map.insert(
                "cniVersions".into(),
                Value::Array(cni_versions.into_iter().map(|v| Value::String(v.into())).collect()),
            );
        }

        map.insert("name".into(), Value::String(self.name.into()));
        map.insert("disableCheck".into(), Value::Bool(self.disable_check));
        map.insert("disableGC".into(), Value::Bool(self.disable_gc));

        let mut plugins: Vec<Value> = Vec::with_capacity(self.plugins.len());
        for plugin in self.plugins.into_iter() {
            plugins.push(plugin.to_json_value()?);
        }
        map.insert("plugins".into(), Value::Array(plugins));

        Ok(Value::Object(map))
    }
}

impl CniSerializable for CniPlugin {
    fn to_json_value(self) -> Result<Value, CniSerializationError> {
        let mut map = Map::new();

        map.insert("type".into(), Value::String(self.plugin_type));
        if let Some(args) = self.args {
            map.insert("args".into(), Value::Object(args));
        }
        if let Some(capabilities) = self.capabilities {
            map.insert("capabilities".into(), Value::Object(capabilities));
        }

        for (key, value) in self.plugin_options {
            if key == "args" || key == "capabilities" || key == "type" {
                return Err(CniSerializationError::OverlappingKey);
            }

            map.insert(key, value);
        }

        Ok(Value::Object(map))
    }
}
