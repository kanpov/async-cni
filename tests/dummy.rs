use std::path::PathBuf;

use tokio_cni::{
    invocation::{CniInvocationArguments, CniInvocationTarget, DirectoryCniLocator, SuCniInvoker},
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

    let arguments = CniInvocationArguments {
        container_id: Some(CniContainerId::new("fcnet").unwrap()),
        net_ns: Some("/var/run/netns/testing".into()),
        interface_name: Some(CniInterfaceName::new("eth0").unwrap()),
        paths: Some(vec![PathBuf::from("/usr/libexec/cni")]),
        attachment: None,
        overridden_cni_version: None,
    };
    let inv_target = CniInvocationTarget::PluginList(&plugin_list);
    dbg!(invoke(CniOperation::Add, &arguments, &inv_target, &invoker, &locator)
        .await
        .unwrap());
    dbg!(
        invoke(CniOperation::Delete, &arguments, &inv_target, &invoker, &locator)
            .await
            .unwrap()
    );
    // let add_inv = CniArguments {
    //     operation: CniOperation::Add,
    //     arguments: CniArguments {
    //         container_id: Some(CniContainerId::new("cnet".into()).unwrap()),
    //         net_ns: Some("/var/run/netns/testing".into()),
    //         interface_name: Some(CniInterfaceName::new("eth0".into()).unwrap()),
    //         paths: Some(vec![PathBuf::from("/usr/libexec/cni")]),
    //         attachment: None,
    //         overridden_cni_version: None,
    //     },
    //     target: CniInvocationTarget::PluginList(&plugin_list),
    //     invoker: Box::new(&invoker),
    //     locator: Box::new(&locator),
    // };
    // let output = dbg!(invoke(add_inv).await.unwrap());

    // let del_inv = CniArguments {
    //     operation: CniOperation::Delete,
    //     arguments: CniArguments {
    //         container_id: Some(CniContainerId::new("cnet".into()).unwrap()),
    //         net_ns: Some("/var/run/netns/testing".into()),
    //         interface_name: Some(CniInterfaceName::new("eth0".into()).unwrap()),
    //         paths: Some(vec![PathBuf::from("/usr/libexec/cni")]),
    //         attachment: Some(output.attachment.unwrap()),
    //         overridden_cni_version: None,
    //     },
    //     target: CniInvocationTarget::PluginList(&plugin_list),
    //     invoker: Box::new(&invoker),
    //     locator: Box::new(&locator),
    // };
    // let _ = invoke(del_inv).await.unwrap();
}
