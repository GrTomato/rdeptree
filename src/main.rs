mod dag;
mod locator;
mod render;
mod utils;

use dag::get_dep_dag_from_env;
use locator::{get_python_interpreter_loc, get_site_packages_loc};
use render::render_dag;
use std::{collections::HashSet, env, process};

/// This function is devoted to parsing and processing of input params
/// This fn will be replaced in future by more convenient framework functionality
fn check_input_params() -> Result<(), &'static str> {
    let args: Vec<String> = env::args().skip(1).collect();

    if args.is_empty() {
        Ok(())
    } else {
        Err("Please just invoker rdeptree with no args")
    }
}

fn main() {
    // step 1: get and validate input params
    if let Err(e) = check_input_params() {
        eprintln!("Incorrect input params: {:?}", e);
        std::process::exit(1);
    }

    // step 2: locate current python env and
    // get location of <site-packages> dir
    let interpreter_loc = get_python_interpreter_loc().unwrap_or_else(|err| {
        eprintln!(
            "ERROR: Can not locate python interpreter location due to an error:\n{:?}",
            err
        );
        std::process::exit(1);
    });

    let path = get_site_packages_loc(&interpreter_loc).unwrap_or_else(|err| {
        eprintln!(
            "ERROR: Can not locate python site-packages location due to an error:\n{:?}",
            err
        );
        std::process::exit(1);
    });

    // TODO: put this into locator
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
