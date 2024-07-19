use std::path::PathBuf;

use tokio_cni::invocation::{CniLocator, DirectoryCniLocator};

#[tokio::test]
async fn t() {
    let locator = DirectoryCniLocator {
        directory_path: PathBuf::from("/usr/libexec/cni"),
        exact_name: true,
    };
    dbg!(locator.locate("tc-redirect-tap").await);
}
