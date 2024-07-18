use std::path::PathBuf;

use async_cni::plugins::{CniDeserializable, CniSerializable, PluginList};
use tokio::fs::read_to_string;

#[tokio::test]
async fn t() {
    let content = read_to_string(PathBuf::from("/home/kanpov/Documents/test.conflist"))
        .await
        .unwrap();
    let pl = PluginList::from_string(content).unwrap();

    dbg!(pl.to_json_value().unwrap());
}
