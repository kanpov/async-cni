use std::path::PathBuf;

use tokio_cni::{
    invocation::{CniInvocation, CniInvocationArguments, CniInvocationTarget, DirectoryCniLocator, SudoCniInvoker},
    invoke,
    plugins::{CniDeserializable, CniPluginList},
    types::{CniContainerId, CniInterfaceName, CniOperation},
};

#[tokio::test]
async fn t() {
    let locator = DirectoryCniLocator {
        directory_path: PathBuf::from("/usr/libexec/cni"),
        exact_name: true,
    };
    let invoker = SudoCniInvoker {
        sudo_path: PathBuf::from("/usr/bin/sudo"),
        password: Some("495762".into()),
    };
    let plugin_list = CniPluginList::from_file(PathBuf::from("/home/kanpov/Documents/test.conflist"))
        .await
        .unwrap();

    let invocation = CniInvocation {
        operation: CniOperation::Add,
        arguments: CniInvocationArguments {
            container_id: Some(CniContainerId::new("fcnet".into()).unwrap()),
            net_ns: Some("/var/run/netns/testing".into()),
            interface_name: Some(CniInterfaceName::new("eth0".into()).unwrap()),
            paths: Some(vec![PathBuf::from("/usr/libexec/cni")]),
            overridden_cni_version: None,
        },
        target: CniInvocationTarget::PluginList(&plugin_list),
        invoker: Box::new(invoker),
        locator: Box::new(locator),
    };
    let result = invoke(invocation).await;
    dbg!(result);
}
