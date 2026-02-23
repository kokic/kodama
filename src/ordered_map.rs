// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Alias Qli (@AliasQli)

/// Ordered map preserving insertion order.
///
/// Backed by `indexmap` to avoid maintaining a custom map implementation.
pub type OrderedMap<K, V> = indexmap::IndexMap<K, V>;
