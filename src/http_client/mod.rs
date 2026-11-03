mod client;
mod user_agent;

pub(crate) use client::build_client;
// Only tests use it today; non-reqwest HTTP stacks need the raw string.
#[allow(unused_imports)]
pub(crate) use user_agent::user_agent;
