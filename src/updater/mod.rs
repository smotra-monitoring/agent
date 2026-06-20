mod checker;
mod downloader;
mod environment;
mod replacer;
mod task;

use checker::is_newer_than_current;

pub use checker::fetch_latest_version;
pub use downloader::download_release_binary;
pub use replacer::replace_binary_and_restart;
pub use task::run_update_checker;
