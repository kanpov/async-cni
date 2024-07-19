use std::path::PathBuf;

use tokio_cni::{
    invocation::{CniInvocation, CniInvocationOverrides, CniInvocationTarget, DirectoryCniLocator, SuCniInvoker},
    invoke,
    plugins::{CniDeserializable, CniPluginList},
    types::{CniContainerId, CniInterfaceName},
};

#[tokio::test]
async fn t() {
    let locator = DirectoryCniLocator {
        directory_path: PathBuf::from("/usr/libexec/cni"),
        exact_name: true,
    };
    let invoker = SuCniInvoker {
        su_path: PathBuf::from("/usr/bin/su"),
        password: "495762".into(),
    };
    let plugin_list = CniPluginList::from_file(PathBuf::from("/home/kanpov/Documents/test.conflist"))
        .await
        .unwrap();

    // let arguments = CniInvocationArguments {
    //     container_id: Some(CniContainerId::new("fcnet").unwrap()),
    //     net_ns: Some("/var/run/netns/testing".into()),
    //     interface_name: Some(CniInterfaceName::new("eth0").unwrap()),
    //     paths: Some(vec![PathBuf::from("/usr/libexec/cni")]),
    //     attachment: None,
    //     overridden_cni_version: None,
    // };
    let invocation_target = CniInvocationTarget::PluginList(&plugin_list);
    let invocation_overrides = CniInvocationOverrides::new();

    let add_inv = invoke(
        CniInvocation::Add {
            container_id: CniContainerId::new("fcnet").unwrap(),
            net_ns: "/var/run/netns/testing".into(),
            interface_name: CniInterfaceName::new("eth0").unwrap(),
            paths: vec![PathBuf::from("/usr/libexec/cni")],
        },
        &invocation_overrides,
        &invocation_target,
        &invoker,
        &locator,
    )
    .await
    .unwrap();
    dbg!(&add_inv);

    let del_inv = invoke(
        CniInvocation::Delete {
            container_id: CniContainerId::new("fcnet").unwrap(),
            interface_name: CniInterfaceName::new("eth0").unwrap(),
            attachment: add_inv.attachment.unwrap(),
            paths: vec![PathBuf::from("/usr/libexec/cni")],
        },
        &invocation_overrides,
        &invocation_target,
        &invoker,
        &locator,
    )
    .await
    .unwrap();
    dbg!(&del_inv);
}
