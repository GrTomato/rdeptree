use regex::Regex;
use std::fs;
use std::fs::{DirEntry, File};
use std::io::{self, BufRead};
use std::path::Path;
use std::{fmt, path::PathBuf};

const DISTRMETA_NAME_REGEX: &'static str = r"Name: (?<name>[a-zA-Z0-9._-]+)";
/// from https://packaging.python.org/en/latest/specifications/name-normalization/#name-normalization
const DISTRMETA_NAME_NORMALIZE_REGEX: &'static str = r"[-_.]+";
const DISTRMETA_VERSION_REGEX: &'static str =
    r"Version: (?<version>\d+(?:(?:\.|!)?(?:dev|post|a|b)?\d+\+?(?:rc|abc)?)+)*";

/// Top-level distribution
#[derive(Debug)]
pub struct DistrMeta {
    pub name: String,
    pub version: String,
}

impl DistrMeta {
    fn normalize_name(name: &str, replace_regex: Regex, replace_to: &str) -> String {
        replace_regex.replace_all(name, replace_to).to_lowercase()
    }

    fn parse_raw_str(name_str: &str, version_str: &str) -> Self {
        // move to use once_cell::sync::Lazy;
        let re_name = Regex::new(DISTRMETA_NAME_REGEX).unwrap();
        let re_version = Regex::new(DISTRMETA_VERSION_REGEX).unwrap();
        let re_name_normalize = Regex::new(DISTRMETA_NAME_NORMALIZE_REGEX).unwrap();

        let name: String = re_name.captures(name_str).unwrap()["name"].parse().unwrap();
        let normalized_name = DistrMeta::normalize_name(&name, re_name_normalize, "-");
        let version: String = re_version.captures(version_str).unwrap()["version"]
            .parse()
            .unwrap();

        Self {
            name: normalized_name,
            version,
        }
    }

    fn from_iter<S>(i: S) -> Result<Self, &'static str>
    where
        S: IntoIterator<Item = String>,
    {
        let filtered_lines: Vec<String> = i
            .into_iter()
            .filter(|line| {
                line.to_lowercase().starts_with("name:")
                    || line.to_lowercase().starts_with("version:")
            })
            .collect();

        if filtered_lines.len() < 2 {
            eprintln!(
                "Unable to parse distr, not enough params: {:?}",
                filtered_lines
            );
            return Err("Unable to parse distr meta, with params");
        }

        Ok(DistrMeta::parse_raw_str(
            &filtered_lines[0],
            &filtered_lines[1],
        ))
    }
}

impl fmt::Display for DistrMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.name, self.version)
    }
}

const METADATA_DIR_SUFFIX: &'static str = ".dist-info";
const METADATA_FILE_NAME: &'static str = "METADATA";

/// from https://doc.rust-lang.org/rust-by-example/std_misc/file/read_lines.html
fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn get_lnreader<P, F>(filename: P, stop_func: F) -> Result<impl Iterator<Item = String>, io::Error>
where
    P: AsRef<Path>,
    F: Fn(&Result<String, std::io::Error>) -> bool,
{
    let line_reader = read_lines(&filename)?;
    Ok(line_reader
        .take_while(move |line| stop_func(line))
        .map(|l| l.unwrap()))
}

/// Get iterator which filter dir entries by metadata suffix
fn get_meta_dirs(env_path: &PathBuf) -> impl Iterator<Item = DirEntry> {
    fs::read_dir(env_path)
        .expect("Can not read site-packages dir")
        .filter_map(|dir_path| match dir_path {
            Ok(dir) => {
                let dir_path_str = dir.file_name();
                if dir_path_str
                    .to_str()
                    .unwrap()
                    .ends_with(METADATA_DIR_SUFFIX)
                {
                    Some(dir)
                } else {
                    None
                }
            }
            Err(_) => None,
        })
}

pub fn get_env_installed_packs(env_path: &PathBuf) -> Result<Vec<DistrMeta>, &'static str> {
    let mut packages_installed: Vec<DistrMeta> = Vec::new();

    for dir in get_meta_dirs(env_path) {
        // get metadata file
        let meta_file_path = dir.path().join(METADATA_FILE_NAME);
        if fs::exists(&meta_file_path).unwrap() {
            // read only first part of the file, until the first empty line
            let read_until_blank = get_lnreader(&meta_file_path, |line| {
                let r = line.as_ref().unwrap();
                // TODO: think about valid delimiter
                !(r == "Description-Content-Type")
            })
            .expect("Can not constuct reader for a file {meta_file_path:?}");

            packages_installed.push(DistrMeta::from_iter(read_until_blank)?);
        }
    }
    Ok(packages_installed)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn distr_meta_from_iter_simple() {
        let sample_meta = [
            String::from("package: some-package"),
            String::from("Name: Sample_Package"),
            String::from("Version: 0.0.1"),
            String::from("Developed by me"),
        ];

        let package_meta = DistrMeta::from_iter(sample_meta.into_iter()).unwrap();

        assert_eq!(package_meta.name, "sample-package");
        assert_eq!(package_meta.version, "0.0.1");
    }

    #[test]
    fn distr_meta_no_version_fail() {
        let sample_meta = [
            String::from("package: some-package"),
            String::from("Name: Sample_Package"),
            String::from("Developed by me"),
        ];

        let result = DistrMeta::from_iter(sample_meta.into_iter());
        assert!(result.is_err());
        assert_eq!(
            result.err(),
            Some("Unable to parse distr meta, with params")
        );
    }

    #[test]
    fn parse_distr_meta_input_str_simple() {
        let name_input = "Name: some-package";
        let expected_name = name_input
            .split(": ")
            .nth(1)
            .expect("Unable to parse name. Check input or regex");

        for version_input in [
            "Version: 2.4.1",
            "Version: 32.445.11",
            "Version: 2014.04",
            "Version: 1.0.15",
            "Version: 1.0",
        ] {
            let actual_value = DistrMeta::parse_raw_str(name_input, version_input);
            let expected_version = version_input
                .split(": ")
                .nth(1)
                .expect("Unable to parse version. Check input or regex");

            assert_eq!(
                actual_value.name, expected_name,
                "Test failed for the pair: actual={}, expected={}",
                actual_value.name, expected_name
            );
            assert_eq!(
                actual_value.version, expected_version,
                "Test failed for the pair: actual={}, expected={}",
                actual_value.version, expected_version
            );
        }
    }

    #[test]
    fn parse_distr_meta_input_str_advanced() {
        let name_input = "Name: there_is-complex--name";

        for version_input in [
            "Version: 1.dev0",
            "Version: 1.0.dev456",
            "Version: 1.0a1",
            "Version: 1.0a2.dev456",
            "Version: 1.0a12.dev456",
            "Version: 1.0a12",
            "Version: 1.0b1.dev456",
            "Version: 1.0b2",
            "Version: 1.0b2.post345.dev456",
            "Version: 1.0b2.post345",
            "Version: 1.0rc1.dev456",
            "Version: 1.0rc1",
            "Version: 1.0+abc.5",
            "Version: 1.0+abc.7",
            "Version: 1.0+5",
            "Version: 1.0.post456.dev34",
            "Version: 1.0.post456",
            "Version: 1.1.dev1",
            "Version: 1!1.0",
        ] {
            let actual_value = DistrMeta::parse_raw_str(name_input, version_input);
            let expected_version = version_input
                .split(": ")
                .nth(1)
                .expect("Unable to parse version. Check input or regex");

            assert_eq!(
                actual_value.name, "there-is-complex-name",
                "Test failed for the pair: actual={}, expected={}",
                actual_value.name, "there-is-complex-name"
            );
            assert_eq!(
                actual_value.version, expected_version,
                "Test failed for the pair: actual={}, expected={}",
                actual_value.version, expected_version
            );
        }
    }
}
