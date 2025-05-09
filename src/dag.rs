use crate::parser::DepParser;
use crate::parser::Rule;
use crate::utils::{get_lnreader, get_meta_dirs};

use pest::Parser;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

fn normalize_name(name: &str, replace_to: &str) -> String {
    let re_name_normalize = Regex::new(DISTRMETA_NAME_NORMALIZE_REGEX).unwrap();
    re_name_normalize
        .replace_all(name, replace_to)
        .to_lowercase()
}

/// from https://packaging.python.org/en/latest/specifications/name-normalization/#name-normalization
const DISTRMETA_NAME_NORMALIZE_REGEX: &'static str = r"[-_.]+";

pub type DistributionName = String;

#[derive(Eq, PartialEq, Hash, Debug)]
pub struct RequiredDistribution {
    pub name: DistributionName,
    pub required_version: String,
}

impl RequiredDistribution {
    fn from_str(name: &str, version: &str) -> Self {
        Self {
            name: normalize_name(name, "-"),
            required_version: version.to_string(),
        }
    }
}

#[derive(Eq, PartialEq, Debug)]
pub struct DistributionMeta {
    pub installed_version: String,
    pub dependencies: HashSet<RequiredDistribution>,
}

impl DistributionMeta {
    fn from_parsed_file(
        installed_version: String,
        dependencies: HashSet<(String, String)>,
    ) -> Result<Self, &'static str> {
        let mut parsed_deps = HashSet::new();
        for (dep_name, version_expr) in dependencies {
            let parse_pair = DepParser::parse(Rule::version_comparison, &version_expr)
                .map_err(|_| "Failed to parse dependency version expression")?
                .next()
                .unwrap();

            parsed_deps.insert(RequiredDistribution::from_str(
                &dep_name,
                parse_pair.as_str(),
            ));
        }

        Ok(Self {
            installed_version,
            dependencies: parsed_deps,
        })
    }
}

pub type DependencyDag = HashMap<DistributionName, DistributionMeta>;

enum ParsedLine {
    Meta(String, String),       // key,value of meta-parameter such as name, version
    Dependency(String, String), // name and parameters of dependency
}

fn parse_line(line: &str) -> Option<ParsedLine> {
    let rules = [
        (
            Rule::distribution_name_row,
            Rule::distribution_name_kw,
            Rule::distribution_name,
        ),
        (
            Rule::distribution_version_row,
            Rule::distribution_version_kw,
            Rule::distribution_version,
        ),
        (
            Rule::required_distribution_row,
            Rule::distribution_name,
            Rule::dependency_str,
        ),
    ];

    for (row_rule, key_rule, value_rule) in rules {
        if let Ok(mut parse_pair) = DepParser::parse(row_rule, line.as_ref()) {
            let inner_pair = parse_pair
                .next()
                .expect("Can not access inner objects for parsed string")
                .into_inner();

            let mut key: String = String::new();
            let mut value: String = String::new();
            for p in inner_pair {
                if p.as_rule() == key_rule {
                    key = p.as_str().to_lowercase();
                }
                if p.as_rule() == value_rule {
                    value = p.as_str().to_string();
                }
            }

            if key.starts_with("name") || key.starts_with("version") {
                return Some(ParsedLine::Meta(key, value));
            } else {
                return Some(ParsedLine::Dependency(key, value));
            }
        }
    }
    None
}

fn node_from_file_iter<I, S>(
    source_iter: I,
) -> Result<(DistributionName, DistributionMeta), &'static str>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let mut name: Option<String> = None;
    let mut version: Option<String> = None;
    let mut dependencies: HashSet<(String, String)> = HashSet::new();

    // iterate over all lines and get parsed strings for required keys
    for line in source_iter {
        if let Some(parsed_line) = parse_line(line.as_ref()) {
            match parsed_line {
                ParsedLine::Meta(k, v) => {
                    if k.starts_with("name") {
                        name = Some(v);
                    } else if k.starts_with("version") {
                        version = Some(v);
                    }
                }
                ParsedLine::Dependency(k, v) => {
                    dependencies.insert((k, v));
                }
            }
        }
    }

    // validate and construnct all the neccesary objects
    let validated_name = normalize_name(&name.ok_or("Can not parse package name from file")?, "-");
    let validated_version = version.ok_or("Can not parse version name from file")?;
    let dm = DistributionMeta::from_parsed_file(validated_version, dependencies)?;

    Ok(((normalize_name(&validated_name, "-")), dm))
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
    fn distr_meta_from_iter_repeating_distrs_different_version() {
        let sample_meta = [
            "package: some-package",
            "Name: Sample_Package",
            "Version: 0.0.1",
            "Developed by me",
            "Requires-Dist: numpy>=1.22.4; python_version < \"3.11\"",
            "Requires-Dist: numpy>=1.23.2; python_version == \"3.11\"",
            "Requires-Dist: numpy>=1.26.0; python_version >= \"3.12\"",
        ];

        let (distribution_name, distribution_meta) =
            node_from_file_iter(sample_meta.into_iter()).unwrap();

        assert_eq!(distribution_name, "sample-package");
        assert_eq!(distribution_meta.installed_version, "0.0.1");
        assert_eq!(distribution_meta.dependencies.is_empty(), false);
        assert_eq!(distribution_meta.dependencies.len(), 3);

        for (depname, depver) in [
            ("numpy", ">=1.22.4"),
            ("numpy", ">=1.23.2"),
            ("numpy", ">=1.26.0"),
        ] {
            let expected_dependency = RequiredDistribution::from_str(depname, depver);
            let actual_dependency = distribution_meta
                .dependencies
                .get(&expected_dependency)
                .expect("Can not find an according dependency");

            assert_eq!(expected_dependency.name, actual_dependency.name);
            assert_eq!(
                expected_dependency.required_version,
                actual_dependency.required_version
            );
        }
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

    #[test]
    fn distr_meta_no_version_fail() {
        let sample_meta = [
            String::from("package: some-package"),
            String::from("Name: Sample_Package"),
            String::from("Developed by me"),
        ];

        let result = node_from_file_iter(sample_meta.into_iter());
        assert!(result.is_err());
        assert_eq!(result.err(), Some("Can not parse version name from file"));
    }

    #[test]
    fn distr_meta_no_name_fail() {
        let sample_meta = [
            String::from("version: 1.0.1"),
            String::from("Developed by me"),
        ];

        let result = node_from_file_iter(sample_meta.into_iter());
        assert!(result.is_err());
        assert_eq!(result.err(), Some("Can not parse package name from file"));
    }

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
            (
                [
                    "Name: simple-name",
                    "Version: 1.0.post456",
                    "Requires-Dist: urllib3 <3,>=1.21.1",
                ],
                ["simple-name", "1.0.post456", "urllib3", "<3,>=1.21.1"],
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
