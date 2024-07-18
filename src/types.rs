use cidr::{IpCidr, IpInet};
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CniAttachment {
    #[serde(rename = "cniVersion")]
    pub cni_version: String,
    pub interfaces: Vec<CniAttachmentInterface>,
    pub ips: Vec<CniAttachmentIp>,
    pub routes: Vec<CniAttachmentRoute>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
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

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CniAttachmentIp {
    pub address: IpCidr,
    pub gateway: IpInet,
    pub interface: u32,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CniAttachmentRoute {
    pub dst: IpCidr,
    pub gw: IpInet,
    pub mtu: u32,
    pub advmss: u32,
    pub priority: u32,
    pub table: u32,
    pub scope: u32,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CniAttachmentDns {
    pub nameservers: Vec<IpInet>,
    pub domain: Option<String>,
    pub search: Vec<String>,
    pub options: Vec<String>,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct CniPluginVersions {
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
