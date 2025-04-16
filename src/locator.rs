use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::{Command, Output};
use std::{env, str};

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

fn execute_command<T>(cmd: T, args: &[&str]) -> Result<Output, std::io::Error>
where
    T: AsRef<OsStr>,
{
    Command::new(cmd).args(args).output()
}

fn run_python_locator_cmd(command: &str) -> Result<Option<Vec<u8>>, std::io::Error> {
    let which_cmd_result = execute_command(command, &["python3"])?;

    let python_interpreter_loc = if which_cmd_result.status.success() {
        Some(which_cmd_result.stdout)
    } else {
        let alt_result = execute_command(command, &["python"])?;
        match alt_result.status.success() {
            true => Some(alt_result.stdout),
            false => {
                eprintln!(
                    "Command <which(where) python(3)> returned: {:?}",
                    String::from_utf8(alt_result.stderr).unwrap()
                );
                None
            }
        }
    };

    Ok(python_interpreter_loc)
}

/// function responsible for identifying the
/// location of current python interpreter
/// Run child sub-proccess using which/where command
///
/// TODO: work out scenario with 2+ paths. Is it possible?
fn get_python_interpreter_location() -> Result<PathBuf, &'static str> {
    let init_command = get_which_command();
    let cmd_result = run_python_locator_cmd(init_command).expect(
        "Unable to locate python interpreter, something went wrong invoking search command",
    );

    if cmd_result.is_none() {
        return Err("Unable to locate python interpreter, command returned nothing");
    }

    let s = String::from_utf8(cmd_result.unwrap())
        .expect("Unable to convert <which(where) python(3)> subcommand result to String");

    Ok(PathBuf::from(s.trim()))
}

fn check_venv_env_var() -> Option<String> {
    if let Ok(e) = env::var("VIRTUAL_ENV") {
        Some(e)
    } else {
        None
    }
}

pub fn get_python_interpreter_loc() -> Result<PathBuf, &'static str> {
    let interpreter_path = match check_venv_env_var() {
        Some(venv_env_val) => {
            let mut pb = PathBuf::from(venv_env_val);
            // TODO: expand find python3 logic
            pb.extend(["bin", "python3"].iter());
            pb
        }
        None => get_python_interpreter_location()?,
    };

    if interpreter_path.exists() {
        Ok(interpreter_path)
    } else {
        eprintln!("Found python interpreter path: {:?}", interpreter_path);
        Err("Found python interpreter path does not exists")
    }
}

/// function responsible for identifying the
/// location of python site-packages dir
pub fn get_site_packages_loc(interpreter_path: &PathBuf) -> Result<PathBuf, &'static str> {
    let command_result_wrapped = execute_command(
        interpreter_path.as_os_str(),
        &[
            "-c",
            r#"import site; print('\n'.join(site.getsitepackages()))"#,
        ],
    );

    let command_result = match command_result_wrapped {
        Ok(val) => {
            if val.status.success() {
                val.stdout
            } else {
                eprintln!(
                    "Command <find python site-packages> returned: {:?}",
                    String::from_utf8(val.stderr).unwrap()
                );
                return Err("Python find site-packages subcommand was unsuccessful");
            }
        }
        Err(e) => {
            eprintln!("{:?}", e);
            return Err("Unable to run `site.getsitepackages()` function in python interpreter to locate site-packages");
        }
    };

    let site_packages_path =
        String::from_utf8(command_result).expect("Unable to convert subcommand result to String");

    let pb = PathBuf::from(site_packages_path.trim());

    if pb.exists() {
        Ok(pb)
    } else {
        eprintln!("Found python site-packages path: {:?}", interpreter_path);
        Err("Found python site-packages path {:?} does not exists")
    }
}
