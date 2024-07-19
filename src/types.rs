use std::vec;

use cidr::{IpCidr, IpInet};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CniOperation {
    Add,
    Delete,
    Check,
    GarbageCollect,
    GetVersions,
    GetStatus,
}

impl AsRef<str> for CniOperation {
    fn as_ref(&self) -> &str {
        match self {
            CniOperation::Add => "ADD",
            CniOperation::Delete => "DEL",
            CniOperation::Check => "CHECK",
            CniOperation::GarbageCollect => "GC",
            CniOperation::GetVersions => "VERSION",
            CniOperation::GetStatus => "STATUS",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CniAttachment {
    #[serde(rename = "cniVersion")]
    pub cni_version: String,
    pub interfaces: Vec<CniAttachmentInterface>,
    pub ips: Vec<CniAttachmentIp>,
    pub routes: Vec<CniAttachmentRoute>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CniAttachmentInterface {
    pub name: String,
    pub mac: Option<String>,
    pub mtu: Option<u32>,
    pub sandbox: String,
    #[serde(rename = "socketPath")]
    pub socket_path: Option<String>,
    #[serde(rename = "pciID")]
    pub pci_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CniAttachmentIp {
    pub address: IpCidr,
    pub gateway: IpInet,
    pub interface: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CniAttachmentRoute {
    pub dst: IpCidr,
    pub gw: IpInet,
    pub mtu: u32,
    pub advmss: u32,
    pub priority: u32,
    pub table: u32,
    pub scope: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CniAttachmentDns {
    pub nameservers: Vec<IpInet>,
    pub domain: Option<String>,
    pub search: Vec<String>,
    pub options: Vec<String>,
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
    pub cni_version: String,
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
    pub fn new(container_id: String) -> Result<CniContainerId, CniValidationError> {
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
pub struct CniNetworkName(String);

impl CniNetworkName {
    pub fn new(network_name: String) -> Result<CniNetworkName, CniValidationError> {
        if network_name.trim().is_empty() {
            return Err(CniValidationError::IsEmptyOrBlank);
        }
        if !network_name.as_bytes().first().unwrap().is_ascii_alphabetic() {
            return Err(CniValidationError::FirstIsNotAlphabetic);
        }

        if !network_name.as_bytes().iter().all(|c| c.is_ascii_alphanumeric()) {
            return Err(CniValidationError::ContainsForbiddenCharacter);
        }

        Ok(CniNetworkName(network_name))
    }
}

impl AsRef<str> for CniNetworkName {
    fn as_ref(&self) -> &str {
        self.0.as_str()
    }
}

impl From<CniNetworkName> for String {
    fn from(value: CniNetworkName) -> Self {
        value.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CniInterfaceName(String);

static IFNAME_MAX_LENGTH: usize = 15;

impl CniInterfaceName {
    pub fn new(interface_name: String) -> Result<CniInterfaceName, CniValidationError> {
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
