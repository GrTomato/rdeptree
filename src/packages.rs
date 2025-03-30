use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::fs::{DirEntry, File};
use std::io::{self, BufRead};
use std::path::Path;
use std::{fmt, path::PathBuf};

const DISTRMETA_NAME_REGEX: &'static str = r"^(?:n|N)ame:(\s)?(?<name>[a-zA-Z0-9._-]+)";
/// from https://packaging.python.org/en/latest/specifications/name-normalization/#name-normalization
const DISTRMETA_NAME_NORMALIZE_REGEX: &'static str = r"[-_.]+";
const DISTRMETA_VERSION_REGEX: &'static str =
    r"^(?:v|V)ersion:(\s)?(?<version>\d+(?:(?:\.|!)?(?:dev|post|a|b)?\d+\+?(?:rc|abc)?)+)*";

/// Top-level distribution
#[derive(Debug)]
pub struct DistrMeta {
    pub name: String,
    pub version: String,
}

impl DistrMeta {
    fn normalize_name(name: &str, replace_to: &str) -> String {
        let re_name_normalize = Regex::new(DISTRMETA_NAME_NORMALIZE_REGEX).unwrap();
        re_name_normalize
            .replace_all(name, replace_to)
            .to_lowercase()
    }

    fn from_iter<I, S>(i: I) -> Result<Self, &'static str>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let field_pattens: HashMap<&str, Regex> = HashMap::from([
            ("name", Regex::new(DISTRMETA_NAME_REGEX).unwrap()),
            ("version", Regex::new(DISTRMETA_VERSION_REGEX).unwrap()),
        ]);

        let filtered_lines: HashMap<&str, String> = i
            .into_iter()
            .filter_map(|line| {
                field_pattens.iter().find_map(|(colname, re)| {
                    if re.is_match(line.as_ref()) {
                        Some((
                            *colname,
                            re.captures(line.as_ref())
                                .unwrap()
                                .name(colname)
                                .unwrap()
                                .as_str()
                                .to_string(),
                        ))
                    } else {
                        None
                    }
                })
            })
            .take(field_pattens.len())
            .collect();

        if filtered_lines.len() < field_pattens.len() {
            eprintln!(
                "Unable to parse distr, not enough params: {:?}",
                filtered_lines
            );
            return Err("Unable to parse distr meta, with params");
        }

        Ok(Self {
            name: DistrMeta::normalize_name(&filtered_lines.get("name").unwrap(), "-"),
            version: filtered_lines.get("version").unwrap().clone(),
        })
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
    fn parse_distr_meta_complex_names() {
        let tests_cases = [
            (["Name: package", "Version: 2.4.1"], "package", "2.4.1"),
            (
                ["Name: some-package", "Version: 32.445.11"],
                "some-package",
                "32.445.11",
            ),
            (
                ["Name: some_package", "Version:2014.04"],
                "some-package",
                "2014.04",
            ),
            (
                ["Name:some_package", "Version: 1.0.15"],
                "some-package",
                "1.0.15",
            ),
            (
                ["Name:there-is_very--complicated_name", "Version: 1.0"],
                "there-is-very-complicated-name",
                "1.0",
            ),
        ];

        for (input_data, expected_name, expected_ver) in tests_cases.iter() {
            let actual_obj = DistrMeta::from_iter(input_data.iter()).unwrap();

            assert_eq!(
                actual_obj.name, *expected_name,
                "Test failed for the pair: actual={}, expected={}",
                actual_obj.name, *expected_name
            );
            assert_eq!(
                actual_obj.version, *expected_ver,
                "Test failed for the pair: actual={}, expected={}",
                actual_obj.version, *expected_ver
            );
        }
    }

    #[test]
    fn parse_distr_meta_complex_version() {
        let tests_cases = [
            (
                ["Name: simple-name", "Version: 1.dev0"],
                "simple-name",
                "1.dev0",
            ),
            (
                ["Name: simple-name", "Version: 1.0.dev456"],
                "simple-name",
                "1.0.dev456",
            ),
            (
                ["Name: simple-name", "Version: 1.0a1"],
                "simple-name",
                "1.0a1",
            ),
            (
                ["Name: simple-name", "Version: 1.0a2.dev456"],
                "simple-name",
                "1.0a2.dev456",
            ),
            (
                ["Name: simple-name", "Version: 1.0a12.dev456"],
                "simple-name",
                "1.0a12.dev456",
            ),
            (
                ["Name: simple-name", "Version: 1.0a12"],
                "simple-name",
                "1.0a12",
            ),
            (
                ["Name: simple-name", "Version: 1.0b1.dev456"],
                "simple-name",
                "1.0b1.dev456",
            ),
            (
                ["Name: simple-name", "Version: 1.0b2"],
                "simple-name",
                "1.0b2",
            ),
            (
                ["Name: simple-name", "Version: 1.0b2.post345.dev456"],
                "simple-name",
                "1.0b2.post345.dev456",
            ),
            (
                ["Name: simple-name", "Version: 1.0b2.post345"],
                "simple-name",
                "1.0b2.post345",
            ),
            (
                ["Name: simple-name", "Version: 1.0rc1.dev456"],
                "simple-name",
                "1.0rc1.dev456",
            ),
            (
                ["Name: simple-name", "Version: 1.0rc1"],
                "simple-name",
                "1.0rc1",
            ),
            (
                ["Name: simple-name", "Version: 1.0+abc.5"],
                "simple-name",
                "1.0+abc.5",
            ),
            (
                ["Name: simple-name", "Version: 1.0+abc.7"],
                "simple-name",
                "1.0+abc.7",
            ),
            (
                ["Name: simple-name", "Version: 1.0+5"],
                "simple-name",
                "1.0+5",
            ),
            (
                ["Name: simple-name", "Version: 1.0.post456.dev34"],
                "simple-name",
                "1.0.post456.dev34",
            ),
            (
                ["Name: simple-name", "Version: 1.0.post456"],
                "simple-name",
                "1.0.post456",
            ),
            (
                ["Name: simple-name", "Version: 1.1.dev1"],
                "simple-name",
                "1.1.dev1",
            ),
            (
                ["Name: simple-name", "Version: 1!1.0"],
                "simple-name",
                "1!1.0",
            ),
        ];

        for (input_data, expected_name, expected_ver) in tests_cases.iter() {
            let actual_obj = DistrMeta::from_iter(input_data.iter()).unwrap();

            assert_eq!(
                actual_obj.name, *expected_name,
                "Test failed for the pair: actual={}, expected={}",
                actual_obj.name, *expected_name
            );
            assert_eq!(
                actual_obj.version, *expected_ver,
                "Test failed for the pair: actual={}, expected={}",
                actual_obj.version, *expected_ver
            );
        }
    }

    #[test]
    fn parse_requires_dist_drop_unmatched_rows() {
        let input_data = [
            "Header: document header",
            "Version: 1.99.1241",
            "NamedRow: ok",
            "Name: pythonDistr",
        ];

        let obj = DistrMeta::from_iter(input_data.iter()).unwrap();

        assert_eq!(obj.name, "pythondistr");
        assert_eq!(obj.version, "1.99.1241");
    }
}
