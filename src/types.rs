use std::{net::IpAddr, str::FromStr, vec};

use cidr::IpInet;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CniOperation {
    Add,
    Delete,
    Check,
    Version,
    Status,
    GarbageCollect,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CniAttachment {
    #[serde(rename = "cniVersion")]
    pub cni_version: CniVersion,
    #[serde(default)]
    pub interfaces: Vec<CniAttachmentInterface>,
    #[serde(default)]
    pub ips: Vec<CniAttachmentIp>,
    #[serde(default)]
    pub routes: Vec<CniAttachmentRoute>,
    pub dns: Option<CniAttachmentDns>,
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
    pub gw: Option<IpAddr>,
    pub mtu: Option<u32>,
    pub advmss: Option<u32>,
    pub priority: Option<u32>,
    pub table: Option<u32>,
    pub scope: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CniAttachmentDns {
    #[serde(default)]
    pub nameservers: Vec<IpAddr>,
    pub domain: Option<String>,
    #[serde(default)]
    pub search: Vec<String>,
    #[serde(default)]
    pub options: Vec<String>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CniVersionObject {
    #[serde(rename = "cniVersion")]
    pub cni_version: CniVersion,
    #[serde(rename = "supportedVersions")]
    pub supported_versions: Vec<CniVersion>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CniError {
    #[serde(rename = "cniVersion")]
    pub cni_version: Option<String>,
    pub code: u16,
    pub msg: String,
    pub details: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CniValidationError {
    IsEmptyOrBlank,
    FirstIsNotAlphabetic,
    ContainsForbiddenCharacter,
    TooLong { maximum_allowed: usize },
    IsForbiddenValue,
    IncorrectSplitAmount,
    SplitMissing,
    SplitNotParseable(<u8 as FromStr>::Err),
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub struct CniValidAttachment {
    #[serde(rename = "containerID")]
    pub container_id: String,
    #[serde(rename = "ifname")]
    pub interface_name: String,
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
    pub fn new(name: impl Into<String>) -> Result<CniName, CniValidationError> {
        let name = name.into();

        if name.trim().is_empty() {
            return Err(CniValidationError::IsEmptyOrBlank);
        }
        if !name.as_bytes().first().unwrap().is_ascii_alphabetic() {
            return Err(CniValidationError::FirstIsNotAlphabetic);
        }

        if !name.as_bytes().iter().all(|c| c.is_ascii_alphanumeric()) {
            return Err(CniValidationError::ContainsForbiddenCharacter);
        }

        Ok(CniName(name))
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
            return Err(CniValidationError::IsForbiddenValue);
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct CniVersion(String);

impl CniVersion {
    fn new(major: u8, minor: u8, patch: u8) -> CniVersion {
        CniVersion(format!("{major}.{minor}.{patch}"))
    }
}

impl AsRef<str> for CniVersion {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl From<CniVersion> for String {
    fn from(value: CniVersion) -> Self {
        value.0
    }
}

impl TryFrom<&str> for CniVersion {
    type Error = CniValidationError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        let splits = value.split('.').collect::<Vec<_>>();
        if splits.len() != 3 {
            return Err(CniValidationError::IncorrectSplitAmount);
        }

        let major = parse_split(&splits, 0)?;
        let minor = parse_split(&splits, 1)?;
        let patch = parse_split(&splits, 2)?;

        Ok(CniVersion::new(major, minor, patch))
    }
}

fn parse_split(splits: &Vec<&str>, index: usize) -> Result<u8, CniValidationError> {
    splits
        .get(index)
        .ok_or(CniValidationError::SplitMissing)?
        .parse()
        .map_err(CniValidationError::SplitNotParseable)
}
