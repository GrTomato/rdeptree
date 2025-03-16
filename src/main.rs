mod locator;
mod packages;
mod render;

use locator::{get_python_interpreter_loc, get_site_packages_loc};
use packages::{get_env_installed_packs, PackageMeta};
use render::render_output;
use std::env;

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

    // step 3: For every METADATA File in given directory
    // Parse base information
    let installed_packs: Vec<PackageMeta> = get_env_installed_packs(&path);
    // step 4: Build some kind of data structure to store dependencies

    // step 5: print results
    render_output(&installed_packs);
}
