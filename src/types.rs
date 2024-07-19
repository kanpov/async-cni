use std::{net::IpAddr, vec};

use cidr::IpInet;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CniAttachment {
    #[serde(rename = "cniVersion")]
    pub cni_version: String,
    pub interfaces: Option<Vec<CniAttachmentInterface>>,
    pub ips: Option<Vec<CniAttachmentIp>>,
    pub routes: Option<Vec<CniAttachmentRoute>>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CniAttachmentInterface {
    pub name: String,
    pub mac: Option<String>,
    pub mtu: Option<u32>,
    pub sandbox: Option<String>,
    #[serde(rename = "socketPath")]
    pub socket_path: Option<String>,
    #[serde(rename = "pciID")]
    pub pci_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CniAttachmentIp {
    pub address: IpInet,
    pub gateway: IpAddr,
    pub interface: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CniAttachmentRoute {
    pub dst: IpInet,
    pub gw: IpAddr,
    pub mtu: u32,
    pub advmss: u32,
    pub priority: u32,
    pub table: u32,
    pub scope: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CniAttachmentDns {
    pub nameservers: Option<Vec<String>>,
    pub domain: Option<String>,
    pub search: Option<Vec<String>>,
    pub options: Option<Vec<String>>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CniVersionObject {
    #[serde(rename = "cniVersion")]
    pub cni_version: String,
    #[serde(rename = "supportedVersions")]
    pub supported_versions: Vec<String>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CniError {
    #[serde(rename = "cniVersion")]
    pub cni_version: Option<String>,
    pub code: u16,
    pub msg: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CniValidationError {
    IsEmptyOrBlank,
    FirstIsNotAlphabetic,
    ContainsForbiddenCharacter,
    TooLong { maximum_allowed: usize },
    IsInvalidValue,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CniContainerId(String);

impl CniContainerId {
    pub fn new(container_id: impl Into<String>) -> Result<CniContainerId, CniValidationError> {
        let container_id = container_id.into();

        if container_id.trim().is_empty() {
            return Err(CniValidationError::IsEmptyOrBlank);
        }
        if !container_id.as_bytes().first().unwrap().is_ascii_alphabetic() {
            return Err(CniValidationError::FirstIsNotAlphabetic);
        }

        let allowed_chars = vec!['.', '_', '-'];
        if !container_id
            .as_bytes()
            .iter()
            .all(|c| c.is_ascii_alphanumeric() || allowed_chars.contains(&(*c as char)))
        {
            return Err(CniValidationError::ContainsForbiddenCharacter);
        }

        Ok(CniContainerId(container_id))
    }
}

impl AsRef<str> for CniContainerId {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl From<CniContainerId> for String {
    fn from(value: CniContainerId) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CniName(String);

impl CniName {
    pub fn new(network_name: impl Into<String>) -> Result<CniName, CniValidationError> {
        let network_name = network_name.into();

        if network_name.trim().is_empty() {
            return Err(CniValidationError::IsEmptyOrBlank);
        }
        if !network_name.as_bytes().first().unwrap().is_ascii_alphabetic() {
            return Err(CniValidationError::FirstIsNotAlphabetic);
        }

        if !network_name.as_bytes().iter().all(|c| c.is_ascii_alphanumeric()) {
            return Err(CniValidationError::ContainsForbiddenCharacter);
        }

        Ok(CniName(network_name))
    }
}

impl AsRef<str> for CniName {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl From<CniName> for String {
    fn from(value: CniName) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CniInterfaceName(String);

static IFNAME_MAX_LENGTH: usize = 15;

impl CniInterfaceName {
    pub fn new(interface_name: impl Into<String>) -> Result<CniInterfaceName, CniValidationError> {
        let interface_name = interface_name.into();

        if interface_name.trim().is_empty() {
            return Err(CniValidationError::IsEmptyOrBlank);
        }
        if interface_name.len() > IFNAME_MAX_LENGTH {
            return Err(CniValidationError::TooLong {
                maximum_allowed: IFNAME_MAX_LENGTH,
            });
        }
        if interface_name == "." || interface_name == ".." {
            return Err(CniValidationError::IsInvalidValue);
        }

        let forbidden_chars = vec![' ', ':', '/'];
        if interface_name
            .as_bytes()
            .iter()
            .any(|c| forbidden_chars.contains(&(*c as char)))
        {
            return Err(CniValidationError::ContainsForbiddenCharacter);
        }

        Ok(CniInterfaceName(interface_name))
    }
}

impl AsRef<str> for CniInterfaceName {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl From<CniInterfaceName> for String {
    fn from(value: CniInterfaceName) -> Self {
        value.0
    }
}
