use anyhow::Result;

use crate::config::AppConfig;
use crate::models::{RawItem, SourceSpec};

pub async fn fetch_x(cfg: &AppConfig, spec: &SourceSpec) -> Result<Vec<RawItem>> {
    crate::browser::fetch_x(cfg, spec).await
}
