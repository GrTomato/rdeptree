use crate::packages::{DependencyDag, DistributionName};

/// Print results of the program, i.e. the list of installed
/// packages and interpreter path
pub fn render_dag(
    dag: &DependencyDag,
    node_name: &DistributionName,
    node_required_ver: Option<&String>,
    level: usize,
) {
    let prefix = "-".repeat(level);

    match dag.get(node_name) {
        Some(val) => {
            if let Some(required_ver) = node_required_ver {
                println!(
                    "{}{} [required={}, installed={}]",
                    prefix, node_name, required_ver, val.installed_version
                )
            } else {
                println!(
                    "{}{} [installed={}]",
                    prefix, node_name, val.installed_version
                );
            }

            for dep in &val.dependencies {
                render_dag(dag, &dep.name, Some(&dep.required_version), level + 4);
            }
        }
        None => return,
    };
}
