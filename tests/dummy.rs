use std::path::PathBuf;

use tokio_cni::{
    invocation::{CniInvocationArguments, CniInvocationTarget, DirectoryCniLocator, SuCniInvoker},
    plugins::{CniDeserializable, CniPluginList},
    runtime::invoke,
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
        password: "495762".to_owned(),
    };
    let plugin_list = CniPluginList::from_file(PathBuf::from("/home/kanpov/Documents/test.conflist"))
        .await
        .unwrap();

    let invocation_target = CniInvocationTarget::PluginList(&plugin_list);
    let mut arguments = CniInvocationArguments::new();
    arguments
        .container_id(CniContainerId::new("fcnet").unwrap())
        .net_ns("/var/run/netns/testing")
        .interface_name(CniInterfaceName::new("eth0").unwrap())
        .paths(vec!["/usr/libexec/cni"]);

    let add_inv = invoke(CniOperation::Add, &arguments, &invocation_target, &invoker, &locator)
        .await
        .unwrap();
    dbg!(&add_inv);
    arguments.attachment(add_inv.attachment.unwrap());

    dbg!(
        invoke(CniOperation::Check, &arguments, &invocation_target, &invoker, &locator)
            .await
            .unwrap()
    );

    let _del_inv = dbg!(
        invoke(CniOperation::Delete, &arguments, &invocation_target, &invoker, &locator,)
            .await
            .unwrap()
    );

    dbg!(invoke(
        CniOperation::Version,
        &arguments,
        &invocation_target,
        &invoker,
        &locator
    )
    .await
    .unwrap());
}
