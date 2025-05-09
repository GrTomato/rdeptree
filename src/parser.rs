use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "rules.pest"] // path to your grammar file
pub struct DepParser;

#[cfg(test)]
mod test {
    use super::*;

    // from https://stackoverflow.com/questions/34662713/how-can-i-create-parameterized-tests-in-rust
    macro_rules! parse_name_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (input, expected) = $value;
                let result = DepParser::parse(Rule::distribution_name_row, input)
                    .expect("Unable to parse name string:\n")
                    .next()
                    .unwrap();
                for pair in result.into_inner() {
                    match pair.as_rule() {
                        Rule::distribution_name_kw => {
                            assert_eq!(pair.as_str(), "Name");
                        }
                        Rule::distribution_name => {
                            assert_eq!(pair.as_str(), expected);
                        }
                        Rule::EOI => (),
                        _other => panic!("Unknown rule to parse: <{:?}>", _other),
                    }
                }
            }
        )*
        }
    }

    parse_name_tests! {
        test_parse_name_simple: ("Name: distribution", "distribution"),
        test_parse_name_camel: ("Name: pythonDistr", "pythonDistr"),
        test_parse_name_snake: ("Name: python_distr", "python_distr"),
        test_parse_name_dash: ("Name: python-distr", "python-distr"),
    }

    macro_rules! parse_version_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (input, expected) = $value;
                let result = DepParser::parse(Rule::distribution_version_row, input)
                    .expect("Unable to parse version string:\n")
                    .next()
                    .unwrap();
                for pair in result.into_inner() {
                    match pair.as_rule() {
                        Rule::distribution_version_kw => {
                            assert_eq!(pair.as_str(), "Version");
                        }
                        Rule::distribution_version => {
                            assert_eq!(pair.as_str(), expected);
                        }
                        Rule::EOI => (),
                        _other => panic!("Unknown rule to parse: <{:?}>", _other),
                    }
                }
            }
        )*
        }
    }

    parse_version_tests! {
        test_parse_version_simple: ("Version: 0.0.1", "0.0.1"),
        test_parse_version_number_multiple_digits: ("Version: 1.99.1241", "1.99.1241"),
        test_parse_version_number_multiple_digits_2: ("Version: 32.445.11", "32.445.11"),
        test_parse_version_number_short: ("Version:2014.04", "2014.04"),
        test_parse_version_number_dev_in_ver: ("Version: 1.dev0", "1.dev0"),
        test_parse_version_number_letter_in_ver: ("Version: 1.0a1", "1.0a1"),
        test_parse_version_number_multiple_letters: ("Version: 1.0a12.dev456", "1.0a12.dev456"),
        test_parse_version_number_multiple_letters_2: ("Version: 1.0b2.post345.dev456", "1.0b2.post345.dev456"),
        test_parse_version_number_multiple_letters_3: ("Version: 1.0rc1.dev456", "1.0rc1.dev456"),
        test_parse_version_number_plus_sign: ("Version: 1.0+abc.5", "1.0+abc.5"),
    }

    macro_rules! parse_required_distribution_tests {
        ($($name:ident: $value:expr,)*) => {
        $(
            #[test]
            fn $name() {
                let (input, expected_name, expected_dependency) = $value;
                let result = DepParser::parse(Rule::required_distribution_row, input)
                    .expect("Unable to parse version string:\n")
                    .next()
                    .unwrap();
                for pair in result.into_inner() {
                    match pair.as_rule() {
                        Rule::required_distribution_kw => {
                            assert_eq!(pair.as_str(), "Requires-Dist:");
                        }
                        Rule::distribution_name => {
                            assert_eq!(pair.as_str(), expected_name);
                        }
                        Rule::dependency_str => {
                            assert_eq!(pair.as_str(), expected_dependency);
                        }
                        Rule::EOI => (),
                        _other => panic!("Unknown rule to parse: <{:?}>", _other),
                    }
                }
            }
        )*
        }
    }

    parse_required_distribution_tests! {
        test_parse_required_distr_simple_eq: ("Requires-Dist: pydantic-core==2.27.2", "pydantic-core", "==2.27.2"),
        test_parse_required_distr_simple_ge: ("Requires-Dist: some_dependency >= 99.123.456", "some_dependency", ">= 99.123.456"),
        test_parse_required_distr_simple_exclamation_version: ("Requires-Dist: some_dependency-package >= 1!1.0", "some_dependency-package", ">= 1!1.0"),
        test_parse_required_distr_simple_complex_version: ("Requires-Dist: some_dependency-package > 1.0b2.post345", "some_dependency-package", "> 1.0b2.post345"),
        test_parse_required_distr_two_versions: ("Requires-Dist: virtualenv<21,>=20.26.4", "virtualenv", "<21,>=20.26.4"),
        test_parse_required_distr_extra_test: ("Requires-Dist: pytest>=8.3.2; extra == 'test'", "pytest", ">=8.3.2; extra == 'test'"),
        test_parse_required_distr_extra_sql_test: ("Requires-Dist: SQLAlchemy>=2.0.0; extra == \"sql-other\"", "SQLAlchemy", ">=2.0.0; extra == \"sql-other\""),
        test_parse_required_distr_python_version: ("Requires-Dist: numpy>=1.22.4; python_version < \"3.11\"", "numpy", ">=1.22.4; python_version < \"3.11\""),
        test_parse_required_distr_extra_package: ("Requires-Dist: pyarrow>=10.0.1; extra == \"pyarrow\"", "pyarrow", ">=10.0.1; extra == \"pyarrow\""),
    }
}
