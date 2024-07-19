use std::path::PathBuf;

use tokio_cni::plugins::{CniDeserializable, CniSerializable, PluginList};

#[tokio::test]
async fn t() {
    let pl = PluginList::from_file(PathBuf::from("/home/kanpov/Documents/test.conflist"))
        .await
        .unwrap();

    dbg!(pl.to_json_value().unwrap());
}
