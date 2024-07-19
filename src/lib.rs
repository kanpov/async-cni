use invocation::{CniInvocation, CniResult};

pub mod invocation;
pub mod plugins;
pub mod types;

pub async fn invoke_cni<'a>(invocation: CniInvocation<'a>) -> CniResult {
    todo!()
}
