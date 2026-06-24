mod checker;
mod downloader;
mod environment;
mod github;
mod replacer;
mod server;

use checker::is_newer_than_current;

pub use checker::fetch_latest_version;
pub use downloader::download_release_binary;
pub use replacer::replace_binary_and_restart;
pub use server::run_update_checker;
