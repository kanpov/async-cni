use std::path::PathBuf;

use tokio_cni::{
    invocation::{CniInvocation, CniInvocationArguments, CniInvocationTarget, DirectoryCniLocator, SuCniInvoker},
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
    let invoker = SuCniInvoker {
        su_path: PathBuf::from("/usr/bin/su"),
        password: "495762".into(),
    };
    let plugin_list = CniPluginList::from_file(PathBuf::from("/home/kanpov/Documents/test.conflist"))
        .await
        .unwrap();

    let add_inv = CniInvocation {
        operation: CniOperation::Add,
        arguments: CniInvocationArguments {
            container_id: Some(CniContainerId::new("nnet".into()).unwrap()),
            net_ns: Some("/var/run/netns/testing".into()),
            interface_name: Some(CniInterfaceName::new("eth1".into()).unwrap()),
            paths: Some(vec![PathBuf::from("/usr/libexec/cni")]),
            attachment: None,
            overridden_cni_version: None,
        },
        target: CniInvocationTarget::PluginList(&plugin_list),
        invoker: Box::new(&invoker),
        locator: Box::new(&locator),
    };
    let output = invoke(add_inv).await.unwrap();

    let del_inv = CniInvocation {
        operation: CniOperation::Add,
        arguments: CniInvocationArguments {
            container_id: Some(CniContainerId::new("fcnet".into()).unwrap()),
            net_ns: Some("/var/run/netns/testing".into()),
            interface_name: Some(CniInterfaceName::new("eth0".into()).unwrap()),
            paths: Some(vec![PathBuf::from("/usr/libexec/cni")]),
            attachment: Some(output.attachment.unwrap()),
            overridden_cni_version: None,
        },
        target: CniInvocationTarget::PluginList(&plugin_list),
        invoker: Box::new(&invoker),
        locator: Box::new(&locator),
    };
    let output = invoke(del_inv).await.unwrap();
    dbg!(output);
}
