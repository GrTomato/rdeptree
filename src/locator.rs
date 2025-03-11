use core::panic;
use std::process::{Command, Output};

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn get_which_command() -> &'static str {
    "which"
}

#[cfg(target_os = "windows")]
fn get_which_command() -> &'static str {
    "where"
}

// The way to break a build if OS is not supported by this module
#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
compile_error!("Unsuported OS! Current build is supported by: [linux, macos, windows].");

fn execute_command(cmd: &str, args: &[&str]) -> Output {
    Command::new(cmd)
        .args(args)
        .output()
        .expect("Error running command {cmd:?} with args {arg:?}")
}

/// function responsible for identifying the
/// location of current python interpreter
/// Run child sub-proccess using which/where command
///
/// TODO: work out scenario with 2+ paths. Is it possible?
fn get_python_interpreter_location() -> String {
    let init_command = get_which_command();
    let init_result = execute_command(init_command, &["python3"]);

    let final_result = if init_result.status.success() {
        init_result.stdout
    } else {
        let alt_result = execute_command(init_command, &["python"]);
        match alt_result.status.success() {
            true => alt_result.stdout,
            false => {
                panic!("No <python3> or <python> alias is set in you env. Please check your local settings")
            }
        }
    };

    String::from_utf8(final_result).expect("Can not convert to String")
}

/// function responsible for identifying the
/// location of python site-packages dir
fn find_python_site_packages_location(interpreter_path: &str) -> String {
    let init_result = execute_command(
        interpreter_path,
        &[
            "-c",
            r#"import site; print('\n'.join(site.getsitepackages()))"#,
        ],
    );

    if init_result.status.success() {
        String::from_utf8(init_result.stdout).expect("Can not convert to String")
    } else {
        panic!("Can not find python site-packages location which error: {init_result:?}")
    }
}

pub fn get_python_dependencies_loc() -> String {
    let python_interpreter_location = get_python_interpreter_location();
    let trimmed_pil = python_interpreter_location.trim();

    let site_packages_location = find_python_site_packages_location(trimmed_pil);
    site_packages_location.trim().to_string()
}
