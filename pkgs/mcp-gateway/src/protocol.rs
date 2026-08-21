use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct Subscription {
    pub server: String,
    pub workspace: PathBuf,
}
