//! Application discovery and caching

mod apps;

pub use apps::{cache_apps, discover_apps, load_cached_apps, AppEntry, IconData};
