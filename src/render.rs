use std::collections::HashSet;

use crate::packages::{DependencyDag, DistributionMeta, DistributionName};

/// Print results of the program, i.e. the list of installed
/// packages and interpreter path
pub fn render_dag(
    dag: &DependencyDag,
    node_name: &DistributionName,
    node_required_ver: Option<&String>,
    level: usize,
) {
    let prefix = "-".repeat(level);
    // This is temporary solution until validation
    // for install packages will be developed
    let meta = match dag.get(node_name) {
        Some(val) => val,
        None => &DistributionMeta {
            installed_version: String::from("Not-installed"),
            dependencies: HashSet::new(),
        },
    };
    if let Some(required_ver) = node_required_ver {
        println!(
            "{}{} [required={}, installed={}]",
            prefix, node_name, required_ver, meta.installed_version
        )
    } else {
        println!(
            "{}{} [installed={}]",
            prefix, node_name, meta.installed_version
        );
    }

    for dep in &meta.dependencies {
        render_dag(dag, &dep.name, Some(&dep.required_version), level + 4);
    }
}
