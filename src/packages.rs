use crate::utils::get_lnreader;

use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::fs::DirEntry;
use std::path::PathBuf;

const DISTRMETA_NAME_REGEX: &'static str = r"^(?:n|N)ame:(\s)?(?<name>[a-zA-Z0-9._-]+)";
/// from https://packaging.python.org/en/latest/specifications/name-normalization/#name-normalization
const DISTRMETA_NAME_NORMALIZE_REGEX: &'static str = r"[-_.]+";
const DISTRMETA_VERSION_REGEX: &'static str =
    r"^(?:v|V)ersion:(\s)?(?<version>\d+(?:(?:\.|!)?(?:dev|post|a|b)?\d+\+?(?:rc|abc)?)+)*";
const DEPDISTRMETA_NAME_REGEX: &'static str = r"^Requires-Dist:(\s)*(?<depname>[a-zA-Z0-9._-]+)(\s)?(\[\w+(?:,\w+)*\])?\s?(\()?(?<depver>(?:(?:,?\s?)?(?:<|<=|!=|==|>=|>|~=|===)+\s?(?:\d[!+\d.a-zA-Z*]+)?)+)?(\))?((?:\s)?;\s.*)?$";

pub type DistributionName = String;

fn normalize_name(name: &str, replace_to: &str) -> String {
    let re_name_normalize = Regex::new(DISTRMETA_NAME_NORMALIZE_REGEX).unwrap();
    re_name_normalize
        .replace_all(name, replace_to)
        .to_lowercase()
}

#[derive(Eq, PartialEq, Hash, Debug)]
pub struct RequiredDistribution {
    pub name: DistributionName,
    pub required_version: String,
}

impl RequiredDistribution {
    fn from_str(name: &str, version: &str) -> Self {
        Self {
            name: name.to_string(),
            required_version: version.to_string(),
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct DistributionMeta {
    pub installed_version: String,
    pub dependencies: HashSet<RequiredDistribution>,
}

pub type DependencyDag = HashMap<DistributionName, DistributionMeta>;

const METADATA_DIR_SUFFIX: &'static str = ".dist-info";
const METADATA_FILE_NAME: &'static str = "METADATA";

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

fn node_from_file_iter<I, S>(i: I) -> Result<(DistributionName, DistributionMeta), &'static str>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let name_regex = Regex::new(DISTRMETA_NAME_REGEX).unwrap();
    let installed_ver_regex = Regex::new(DISTRMETA_VERSION_REGEX).unwrap();
    let dependency_regex = Regex::new(DEPDISTRMETA_NAME_REGEX).unwrap();

    let mut name = String::new();
    let mut installed_ver = String::new();
    let mut dependencies: HashSet<RequiredDistribution> = HashSet::new();

    for line in i {
        if name_regex.is_match(line.as_ref()) {
            let non_normalized_name = name_regex
                .captures(line.as_ref())
                .unwrap()
                .name("name")
                .unwrap()
                .as_str();
            name = normalize_name(non_normalized_name, "-");
        } else if installed_ver_regex.is_match(line.as_ref()) {
            installed_ver = installed_ver_regex
                .captures(line.as_ref())
                .unwrap()
                .name("version")
                .unwrap()
                .as_str()
                .to_string();
        } else if dependency_regex.is_match(line.as_ref()) {
            let captures = dependency_regex.captures(line.as_ref()).unwrap();
            let norm_name = normalize_name(captures.name("depname").unwrap().as_str(), "-");
            dependencies.insert(RequiredDistribution::from_str(
                &norm_name,
                captures
                    .name("depver")
                    .map(|val| val.as_str())
                    .unwrap_or("Any"),
            ));
        }
    }

    let dm = DistributionMeta {
        installed_version: installed_ver,
        dependencies: dependencies,
    };

    Ok((name, dm))
}

pub fn get_dep_dag_from_env(env_path: &PathBuf) -> Result<DependencyDag, &'static str> {
    let mut dependency_dag: DependencyDag = HashMap::new();

    for dir in get_meta_dirs(env_path) {
        // get metadata file
        let meta_file_path = dir.path().join(METADATA_FILE_NAME);
        if fs::exists(&meta_file_path).unwrap() {
            // read only first part of the file, until the first stopper
            let readline_iter = get_lnreader(&meta_file_path, |line| {
                let r = line.as_ref().unwrap();
                // TODO: think about valid delimiter
                !(r == "Description-Content-Type")
            })
            .expect("Can not constuct reader for a file {meta_file_path:?}");

            let (k, v) = node_from_file_iter(readline_iter)?;
            dependency_dag.insert(k, v);
        }
    }
    Ok(dependency_dag)
}

// #[cfg(test)]
// mod test {
//     use super::*;

//     #[test]
//     fn distr_meta_from_iter_simple() {
//         let sample_meta = [
//             String::from("package: some-package"),
//             String::from("Name: Sample_Package"),
//             String::from("Version: 0.0.1"),
//             String::from("Developed by me"),
//         ];

//         let package_meta = dag_node_from_file_iter(sample_meta.into_iter()).unwrap();

//         assert_eq!(package_meta.name, "sample-package");
//         assert_eq!(package_meta.version, "0.0.1");
//     }

//     #[test]
//     fn distr_meta_no_version_fail() {
//         let sample_meta = [
//             String::from("package: some-package"),
//             String::from("Name: Sample_Package"),
//             String::from("Developed by me"),
//         ];

//         let result = dag_node_from_file_iter(sample_meta.into_iter());
//         assert!(result.is_err());
//         assert_eq!(result.err(), Some("Distr meta missing required fields"));
//     }

//     #[test]
//     fn parse_distr_meta_complex_names() {
//         let tests_cases = [
//             (["Name: package", "Version: 2.4.1"], "package", "2.4.1"),
//             (
//                 ["Name: some-package", "Version: 32.445.11"],
//                 "some-package",
//                 "32.445.11",
//             ),
//             (
//                 ["Name: some_package", "Version:2014.04"],
//                 "some-package",
//                 "2014.04",
//             ),
//             (
//                 ["Name:some_package", "Version: 1.0.15"],
//                 "some-package",
//                 "1.0.15",
//             ),
//             (
//                 ["Name:there-is_very--complicated_name", "Version: 1.0"],
//                 "there-is-very-complicated-name",
//                 "1.0",
//             ),
//         ];

//         for (input_data, expected_name, expected_ver) in tests_cases.iter() {
//             let actual_obj = dag_node_from_file_iter(input_data.iter()).unwrap();

//             assert_eq!(
//                 actual_obj.name, *expected_name,
//                 "Test failed for the pair: actual={}, expected={}",
//                 actual_obj.name, *expected_name
//             );
//             assert_eq!(
//                 actual_obj.version, *expected_ver,
//                 "Test failed for the pair: actual={}, expected={}",
//                 actual_obj.version, *expected_ver
//             );
//         }
//     }

//     #[test]
//     fn parse_distr_meta_complex_version() {
//         let tests_cases = [
//             (
//                 ["Name: simple-name", "Version: 1.dev0"],
//                 "simple-name",
//                 "1.dev0",
//             ),
//             (
//                 ["Name: simple-name", "Version: 1.0.dev456"],
//                 "simple-name",
//                 "1.0.dev456",
//             ),
//             (
//                 ["Name: simple-name", "Version: 1.0a1"],
//                 "simple-name",
//                 "1.0a1",
//             ),
//             (
//                 ["Name: simple-name", "Version: 1.0a2.dev456"],
//                 "simple-name",
//                 "1.0a2.dev456",
//             ),
//             (
//                 ["Name: simple-name", "Version: 1.0a12.dev456"],
//                 "simple-name",
//                 "1.0a12.dev456",
//             ),
//             (
//                 ["Name: simple-name", "Version: 1.0a12"],
//                 "simple-name",
//                 "1.0a12",
//             ),
//             (
//                 ["Name: simple-name", "Version: 1.0b1.dev456"],
//                 "simple-name",
//                 "1.0b1.dev456",
//             ),
//             (
//                 ["Name: simple-name", "Version: 1.0b2"],
//                 "simple-name",
//                 "1.0b2",
//             ),
//             (
//                 ["Name: simple-name", "Version: 1.0b2.post345.dev456"],
//                 "simple-name",
//                 "1.0b2.post345.dev456",
//             ),
//             (
//                 ["Name: simple-name", "Version: 1.0b2.post345"],
//                 "simple-name",
//                 "1.0b2.post345",
//             ),
//             (
//                 ["Name: simple-name", "Version: 1.0rc1.dev456"],
//                 "simple-name",
//                 "1.0rc1.dev456",
//             ),
//             (
//                 ["Name: simple-name", "Version: 1.0rc1"],
//                 "simple-name",
//                 "1.0rc1",
//             ),
//             (
//                 ["Name: simple-name", "Version: 1.0+abc.5"],
//                 "simple-name",
//                 "1.0+abc.5",
//             ),
//             (
//                 ["Name: simple-name", "Version: 1.0+abc.7"],
//                 "simple-name",
//                 "1.0+abc.7",
//             ),
//             (
//                 ["Name: simple-name", "Version: 1.0+5"],
//                 "simple-name",
//                 "1.0+5",
//             ),
//             (
//                 ["Name: simple-name", "Version: 1.0.post456.dev34"],
//                 "simple-name",
//                 "1.0.post456.dev34",
//             ),
//             (
//                 ["Name: simple-name", "Version: 1.0.post456"],
//                 "simple-name",
//                 "1.0.post456",
//             ),
//             (
//                 ["Name: simple-name", "Version: 1.1.dev1"],
//                 "simple-name",
//                 "1.1.dev1",
//             ),
//             (
//                 ["Name: simple-name", "Version: 1!1.0"],
//                 "simple-name",
//                 "1!1.0",
//             ),
//         ];

//         for (input_data, expected_name, expected_ver) in tests_cases.iter() {
//             let actual_obj = dag_node_from_file_iter(input_data.iter()).unwrap();

//             assert_eq!(
//                 actual_obj.name, *expected_name,
//                 "Test failed for the pair: actual={}, expected={}",
//                 actual_obj.name, *expected_name
//             );
//             assert_eq!(
//                 actual_obj.version, *expected_ver,
//                 "Test failed for the pair: actual={}, expected={}",
//                 actual_obj.version, *expected_ver
//             );
//         }
//     }

//     #[test]
//     fn parse_requires_dist_drop_unmatched_rows() {
//         let input_data = [
//             "Header: document header",
//             "Version: 1.99.1241",
//             "NamedRow: ok",
//             "Name: pythonDistr",
//         ];

//         let obj = dag_node_from_file_iter(input_data.iter()).unwrap();

//         assert_eq!(obj.name, "pythondistr");
//         assert_eq!(obj.version, "1.99.1241");
//     }
// }
