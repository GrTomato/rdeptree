mod locator;
mod packages;
mod render;
mod utils;

use locator::get_python_dependencies_loc;
use packages::get_dep_dag_from_env;
use render::render_dag;
use std::{collections::HashSet, env, process};

/// This part is devoted to parsing and processing of input params
/// This fn will be replaced in future by more convenient framework functionality
fn check_input_params() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 1 {
        eprintln!("Please just invoker rdeptree with no args");
        std::process::exit(1);
    }
}

fn main() {
    // step 1: get and validate input params
    check_input_params();

    // step 2: locate current python env and
    // get location of <site-packages> dir
    let path = get_python_dependencies_loc();

    if !path.exists() {
        eprintln!("Path must point to an existing entity");
    }

    // step 3: parse metadata to dag
    // Parse base information
    let dag = get_dep_dag_from_env(&path).unwrap_or_else(|err| {
        eprintln!("Problem parsing installed distributions: {err}");
        process::exit(1);
    });

    let non_empty_dependenices_names: HashSet<&String> = dag
        .values()
        .into_iter()
        .filter_map(|v| {
            if !v.dependencies.is_empty() {
                Some(&v.dependencies)
            } else {
                None
            }
        })
        .flatten()
        .map(|v| &v.name)
        .collect();

    let top_level_distributions: Vec<&String> = dag
        .keys()
        .into_iter()
        .filter_map(|k| {
            if !non_empty_dependenices_names.contains(k) {
                Some(k)
            } else {
                None
            }
        })
        .collect();

    // step 5: print results
    for tlp in top_level_distributions {
        render_dag(&dag, tlp, None, 0);
    }
}
