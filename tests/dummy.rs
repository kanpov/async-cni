use std::{collections::HashMap, path::PathBuf};

use tokio_cni::invocation::{CniInvoker, SudoCniInvoker};

#[tokio::test]
async fn t() {
    let ivk = SudoCniInvoker {
        sudo_path: PathBuf::from("/usr/bin/sudo"),
        password: Some("495762".into()),
    };
    let output = dbg!(ivk
        .invoke(PathBuf::from("/usr/bin/ls").as_ref(), HashMap::new(), "".into())
        .await
        .unwrap());
}
