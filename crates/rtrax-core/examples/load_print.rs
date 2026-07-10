//! Phase 0 smoke test: load a module, print metadata, exit.
//!
//! Usage: `cargo run --example load_print -- path/to/song.xm`

use anyhow::{bail, Context, Result};
use openmpt::module::{metadata::MetadataKey, Logger, Module};
use std::fs::File;

fn main() -> Result<()> {
    let path = std::env::args()
        .nth(1)
        .context("usage: load_print <module-file>")?;

    let mut file = File::open(&path).with_context(|| format!("opening {path}"))?;

    let mut module = Module::create(&mut file, Logger::None, &[])
        .map_err(|_| anyhow::anyhow!("libopenmpt could not parse {path}"))?;

    let title = module
        .get_metadata(MetadataKey::ModuleTitle)
        .unwrap_or_default();
    let type_long = module
        .get_metadata(MetadataKey::TypeName)
        .unwrap_or_default();
    let tracker = module
        .get_metadata(MetadataKey::ModuleTracker)
        .unwrap_or_default();

    let channels = module.get_num_channels();
    let orders = module.get_num_orders();
    let patterns = module.get_num_patterns();
    let duration = module.get_duration_seconds();

    println!("file:      {path}");
    println!("title:     {title}");
    println!("format:    {type_long}");
    println!("tracker:   {tracker}");
    println!("channels:  {channels}");
    println!("orders:    {orders}");
    println!("patterns:  {patterns}");
    println!("duration:  {:.2}s ({})", duration, fmt_mmss(duration));

    if channels < 1 {
        bail!("module reports 0 channels — likely corrupt");
    }

    Ok(())
}

fn fmt_mmss(secs: f64) -> String {
    let total = secs.max(0.0) as u32;
    format!("{:02}:{:02}", total / 60, total % 60)
}
