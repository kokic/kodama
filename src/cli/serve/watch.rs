// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic), Spore (@s-cerevisiae)

mod analysis;
mod runtime;
mod strategy;

pub(super) use analysis::{
    analyze_watch_changes, format_watch_change_stats, should_restart_for_config_change,
};
pub(super) use runtime::watch_paths;
pub(super) use strategy::compose_watched_paths;
