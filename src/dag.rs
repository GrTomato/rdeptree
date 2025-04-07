use crate::utils::{get_lnreader, get_meta_dirs};

use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

/// from https://packaging.python.org/en/latest/specifications/name-normalization/#name-normalization
const DISTRMETA_NAME_NORMALIZE_REGEX: &'static str = r"[-_.]+";

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

const DISTRMETA_NAME_REGEX: &'static str = r"^(?:n|N)ame:(\s)?(?<name>[a-zA-Z0-9._-]+)";
const DISTRMETA_VERSION_REGEX: &'static str =
    r"^(?:v|V)ersion:(\s)?(?<version>\d+(?:(?:\.|!)?(?:dev|post|a|b)?\d+\+?(?:rc|abc)?)+)*";
const DEPDISTRMETA_NAME_REGEX: &'static str = r"^Requires-Dist:(\s)*(?<depname>[a-zA-Z0-9._-]+)(\s)?(\[\w+(?:,\w+)*\])?\s?(\()?(?<depver>(?:(?:,?\s?)?(?:<|<=|!=|==|>=|>|~=|===)+\s?(?:\d[!+\d.a-zA-Z*]+)?)+)?(\))?((?:\s)?;\s.*)?$";

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

const METADATA_FILE_NAME: &'static str = "METADATA";

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

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn distr_meta_from_iter_simple() {
        let sample_meta = [
            "package: some-package",
            "Name: Sample_Package",
            "Version: 0.0.1",
            "Developed by me",
            "Requires-Dist: pyarrow>=10.0.1; extra == \"pyarrow\"",
        ];

        let (distribution_name, distribution_meta) =
            node_from_file_iter(sample_meta.into_iter()).unwrap();

        assert_eq!(distribution_name, "sample-package");
        assert_eq!(distribution_meta.installed_version, "0.0.1");
        assert_eq!(distribution_meta.dependencies.is_empty(), false);
        assert_eq!(distribution_meta.dependencies.len(), 1);

        let expected_dependency = RequiredDistribution::from_str("pyarrow", ">=10.0.1");
        let actual_dependency = distribution_meta
            .dependencies
            .get(&expected_dependency)
            .unwrap();

        assert_eq!(expected_dependency.name, actual_dependency.name);
        assert_eq!(
            expected_dependency.required_version,
            actual_dependency.required_version
        );
    }

    #[test]
    fn parse_requires_dist_drop_unmatched_records() {
        let input_data = [
            "Header: document header",
            "Version: 1.99.1241",
            "NamedRow: ok",
            "Name: pythonDistr",
            "Requires-Dist: dependency_package == 1.0.1",
        ];

        let (distribution_name, distribution_meta) =
            node_from_file_iter(input_data.iter()).unwrap();

        assert_eq!(distribution_name, "pythondistr");
        assert_eq!(distribution_meta.installed_version, "1.99.1241");
        assert_eq!(distribution_meta.dependencies.len(), 1);

        let expected_dependency = RequiredDistribution::from_str("dependency-package", "== 1.0.1");
        let actual_dependency = distribution_meta
            .dependencies
            .get(&expected_dependency)
            .unwrap();

        assert_eq!(expected_dependency.name, actual_dependency.name);
        assert_eq!(
            expected_dependency.required_version,
            actual_dependency.required_version
        );
    }

    #[test]
    fn parse_multiple_dependencies() {
        let input_data = [
            "Header: document header",
            "Version: 1.99.1241",
            "NamedRow: ok",
            "Name: pythonDistr",
            "Requires-Dist: dependency_package == 1.0.1",
            "Requires-Dist: some_dependency >= 99.123.456",
        ];

        let (distribution_name, distribution_meta) =
            node_from_file_iter(input_data.iter()).unwrap();

        assert_eq!(distribution_name, "pythondistr");
        assert_eq!(distribution_meta.installed_version, "1.99.1241");
        assert_eq!(distribution_meta.dependencies.len(), 2);

        for (depname, depver) in [
            ("dependency-package", "== 1.0.1"),
            ("some-dependency", ">= 99.123.456"),
        ] {
            let expected_dependency = RequiredDistribution::from_str(depname, depver);
            let actual_dependency = distribution_meta
                .dependencies
                .get(&expected_dependency)
                .unwrap();

            assert_eq!(expected_dependency.name, actual_dependency.name);
            assert_eq!(
                expected_dependency.required_version,
                actual_dependency.required_version
            );
        }
    }

    // #[test]
    // fn distr_meta_no_version_fail() {
    //     let sample_meta = [
    //         String::from("package: some-package"),
    //         String::from("Name: Sample_Package"),
    //         String::from("Developed by me"),
    //     ];

    //     let result = node_from_file_iter(sample_meta.into_iter());
    //     assert!(dbg!(result).is_err());
    //     // assert_eq!(result.err(), Some("Distr meta missing required fields"));
    // }

    #[test]
    fn parse_distr_meta_complex_names() {
        let tests_cases = [
            (
                [
                    "Name: package",
                    "Version: 2.4.1",
                    "Requires-Dist: dependency_package == 1.0.1",
                ],
                ["package", "2.4.1", "dependency-package", "== 1.0.1"],
            ),
            (
                [
                    "Name: some-package",
                    "Version: 32.445.11",
                    "Requires-Dist: some_dependency-package >= 3.3.3",
                ],
                [
                    "some-package",
                    "32.445.11",
                    "some-dependency-package",
                    ">= 3.3.3",
                ],
            ),
            (
                [
                    "Name: some_package",
                    "Version:2014.04",
                    "Requires-Dist: some_dependency-package != 0.5.999",
                ],
                [
                    "some-package",
                    "2014.04",
                    "some-dependency-package",
                    "!= 0.5.999",
                ],
            ),
            (
                [
                    "Name:there-is_very--complicated_name",
                    "Version: 1.0",
                    "Requires-Dist: there-is_very--complicated_DEPENDENCY_-_-name != 0.5.999",
                ],
                [
                    "there-is-very-complicated-name",
                    "1.0",
                    "there-is-very-complicated-dependency-name",
                    "!= 0.5.999",
                ],
            ),
        ];

        for (input_data, expected_data) in tests_cases.iter() {
            let (distribution_name, distribution_meta) =
                node_from_file_iter(input_data.iter()).unwrap();

            assert_eq!(
                distribution_name, expected_data[0],
                "Test failed for the pair: actual={}, expected={}",
                distribution_name, expected_data[0],
            );
            assert_eq!(
                distribution_meta.installed_version, expected_data[1],
                "Test failed for the pair: actual={}, expected={}",
                distribution_meta.installed_version, expected_data[1],
            );

            assert_eq!(distribution_meta.dependencies.len(), 1);

            let expected_dependency =
                RequiredDistribution::from_str(expected_data[2], expected_data[3]);
            let actual_dependency = &distribution_meta
                .dependencies
                .get(&expected_dependency)
                .expect("FAIL: There is no same object as expected");

            assert_eq!(
                expected_dependency.name, actual_dependency.name,
                "Test failed for the pair: actual={}, expected={}",
                expected_dependency.name, actual_dependency.name,
            );
            assert_eq!(
                expected_dependency.required_version, actual_dependency.required_version,
                "Test failed for the pair: actual={}, expected={}",
                expected_dependency.required_version, actual_dependency.required_version,
            );
        }
    }

    #[test]
    fn parse_distr_meta_complex_version() {
        let tests_cases = [
            (
                [
                    "Name: simple-name",
                    "Version: 1.dev0",
                    "Requires-Dist: some_dependency-package != 1.0.dev456",
                ],
                [
                    "simple-name",
                    "1.dev0",
                    "some-dependency-package",
                    "!= 1.0.dev456",
                ],
            ),
            (
                [
                    "Name: simple-name",
                    "Version: 1.0a1",
                    "Requires-Dist: some_dependency-package < 1.0a2.dev456",
                ],
                [
                    "simple-name",
                    "1.0a1",
                    "some-dependency-package",
                    "< 1.0a2.dev456",
                ],
            ),
            (
                [
                    "Name: simple-name",
                    "Version: 1.0a12.dev456",
                    "Requires-Dist: some_dependency-package > 1.0a12",
                ],
                [
                    "simple-name",
                    "1.0a12.dev456",
                    "some-dependency-package",
                    "> 1.0a12",
                ],
            ),
            (
                [
                    "Name: simple-name",
                    "Version: 1.0b1.dev456",
                    "Requires-Dist: some_dependency-package <= 1.0b2",
                ],
                [
                    "simple-name",
                    "1.0b1.dev456",
                    "some-dependency-package",
                    "<= 1.0b2",
                ],
            ),
            (
                [
                    "Name: simple-name",
                    "Version: 1.0b2.post345.dev456",
                    "Requires-Dist: some_dependency-package > 1.0b2.post345",
                ],
                [
                    "simple-name",
                    "1.0b2.post345.dev456",
                    "some-dependency-package",
                    "> 1.0b2.post345",
                ],
            ),
            (
                [
                    "Name: simple-name",
                    "Version: 1.0rc1.dev456",
                    "Requires-Dist: some_dependency-package != 1.0rc1",
                ],
                [
                    "simple-name",
                    "1.0rc1.dev456",
                    "some-dependency-package",
                    "!= 1.0rc1",
                ],
            ),
            (
                [
                    "Name: simple-name",
                    "Version: 1.0+abc.5",
                    "Requires-Dist: some_dependency-package < 1.0+abc.7",
                ],
                [
                    "simple-name",
                    "1.0+abc.5",
                    "some-dependency-package",
                    "< 1.0+abc.7",
                ],
            ),
            (
                [
                    "Name: simple-name",
                    "Version: 1.0+5",
                    "Requires-Dist: some_dependency-package >= 1.0.post456.dev34",
                ],
                [
                    "simple-name",
                    "1.0+5",
                    "some-dependency-package",
                    ">= 1.0.post456.dev34",
                ],
            ),
            (
                [
                    "Name: simple-name",
                    "Version: 1.0.post456",
                    "Requires-Dist: some_dependency-package >= 1!1.0",
                ],
                [
                    "simple-name",
                    "1.0.post456",
                    "some-dependency-package",
                    ">= 1!1.0",
                ],
            ),
        ];

        for (input_data, expected_data) in tests_cases.iter() {
            let (distribution_name, distribution_meta) =
                node_from_file_iter(input_data.iter()).unwrap();

            assert_eq!(
                distribution_name, expected_data[0],
                "Test failed for the pair: actual={}, expected={}",
                distribution_name, expected_data[0],
            );
            assert_eq!(
                distribution_meta.installed_version, expected_data[1],
                "Test failed for the pair: actual={}, expected={}",
                distribution_meta.installed_version, expected_data[1],
            );

            assert_eq!(distribution_meta.dependencies.len(), 1);

            let expected_dependency =
                RequiredDistribution::from_str(expected_data[2], expected_data[3]);
            let actual_dependency = &distribution_meta
                .dependencies
                .get(&expected_dependency)
                .expect("FAIL: There is no same object as expected");

            assert_eq!(
                expected_dependency.name, actual_dependency.name,
                "Test failed for the pair: actual={}, expected={}",
                expected_dependency.name, actual_dependency.name,
            );
            assert_eq!(
                expected_dependency.required_version, actual_dependency.required_version,
                "Test failed for the pair: actual={}, expected={}",
                expected_dependency.required_version, actual_dependency.required_version,
            );
        }
    }
}
