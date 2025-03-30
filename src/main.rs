mod locator;
mod packages;
mod render;
mod utils;

use locator::get_python_dependencies_loc;
use packages::{get_env_installed_packs, DistrMeta};
use render::render_output;
use std::{env, process};

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

    // step 3: For every METADATA File in given directory
    // Parse base information
    let installed_packs: Vec<DistrMeta> = get_env_installed_packs(&path).unwrap_or_else(|err| {
        eprintln!("Problem parsing installed distributions: {err}");
        process::exit(1);
    });
    // step 4: Build some kind of data structure to store dependencies

    // step 5: print results
    render_output(&installed_packs);
}
