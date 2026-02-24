// Copyright (c) 2025 Kodama Project. All rights reserved.
// Released under the GPL-3.0 license as described in the file LICENSE.
// Authors: Kokic (@kokic)

use crate::compiler::CompileOutputs;

#[derive(clap::Args, Clone, Copy, Debug, Default)]
pub struct OutputControlArgs {
    /// Generate "kodama.json" ("build": default on, "serve": default off).
    #[arg(long, default_value_t = false, conflicts_with = "no_indexes")]
    pub indexes: bool,

    /// Skip generating "kodama.json".
    #[arg(long, default_value_t = false, conflicts_with = "indexes")]
    pub no_indexes: bool,

    /// Generate "kodama.graph.json" ("build": default on, "serve": default off).
    #[arg(long, default_value_t = false, conflicts_with = "no_graph")]
    pub graph: bool,

    /// Skip generating "kodama.graph.json".
    #[arg(long, default_value_t = false, conflicts_with = "graph")]
    pub no_graph: bool,
}

impl OutputControlArgs {
    pub fn resolve(self, defaults: CompileOutputs) -> CompileOutputs {
        let indexes = if self.indexes {
            true
        } else if self.no_indexes {
            false
        } else {
            defaults.indexes
        };

        let graph = if self.graph {
            true
        } else if self.no_graph {
            false
        } else {
            defaults.graph
        };

        CompileOutputs { indexes, graph }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_output_control_uses_defaults_when_unspecified() {
        let args = OutputControlArgs::default();
        let outputs = args.resolve(CompileOutputs {
            indexes: true,
            graph: false,
        });
        assert!(outputs.indexes);
        assert!(!outputs.graph);
    }

    #[test]
    fn test_output_control_enables_outputs() {
        let args = OutputControlArgs {
            indexes: true,
            no_indexes: false,
            graph: true,
            no_graph: false,
        };
        let outputs = args.resolve(CompileOutputs {
            indexes: false,
            graph: false,
        });
        assert!(outputs.indexes);
        assert!(outputs.graph);
    }

    #[test]
    fn test_output_control_disables_outputs() {
        let args = OutputControlArgs {
            indexes: false,
            no_indexes: true,
            graph: false,
            no_graph: true,
        };
        let outputs = args.resolve(CompileOutputs {
            indexes: true,
            graph: true,
        });
        assert!(!outputs.indexes);
        assert!(!outputs.graph);
    }
}
