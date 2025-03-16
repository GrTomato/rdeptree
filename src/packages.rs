use std::collections::HashMap;
use std::fs;
use std::fs::{DirEntry, File};
use std::io::{self, BufRead};
use std::path::Path;
use std::{fmt, path::PathBuf};

#[derive(Debug)]
pub struct PackageMeta {
    pub name: String,
    pub version: String,
    // pub dependencies: Vec<String>,
}

impl PackageMeta {
    fn from_iter<S>(i: S) -> Self
    where
        S: IntoIterator<Item = String>,
    {
        let filtered_lines: Vec<String> = i
            .into_iter()
            .filter(|line| {
                let pos = line.chars().position(|c| c == ':').unwrap_or(0);
                &line[0..pos].to_lowercase() == "name" || &line[0..pos].to_lowercase() == "version"
                // || &line[0..pos].to_lowercase() == "requires-dist"
            })
            .collect();

        let mut constructor_map: HashMap<String, String> = HashMap::new();

        for line in filtered_lines {
            let (k, v) = line.split_once(": ").unwrap();

            if k.to_lowercase() == "requires-dist" {
                match constructor_map.get("requires-dist") {
                    Some(map_val) => {
                        let curr_val = v.to_string();
                        constructor_map
                            .insert(k.to_lowercase().to_string(), curr_val + "|" + map_val);
                    }
                    None => {
                        constructor_map.insert(k.to_lowercase().to_string(), v.to_string());
                    }
                };
            } else {
                constructor_map.insert(k.to_lowercase().to_string(), v.to_string());
            }
        }

        Self {
            name: constructor_map.get("name").unwrap().clone(),
            version: constructor_map.get("version").unwrap().clone(),
            // dependencies: match constructor_map.get("requires-dist") {
            //     Some(v) => v.split("|").map(|v| v.to_string()).collect(),
            //     None => vec![],
            // },
        }
    }
}

impl fmt::Display for PackageMeta {
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

pub fn get_env_installed_packs(env_path: &PathBuf) -> Vec<PackageMeta> {
    let mut packages_installed: Vec<PackageMeta> = Vec::new();

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

            packages_installed.push(PackageMeta::from_iter(read_until_blank));
        }
    }
    packages_installed
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn package_meta_from_iter_success() {
        let sample_meta = [
            String::from("package: some-package"),
            String::from("Name: Sample_Package"),
            String::from("Version: v0.0.1"),
            String::from("Developed by me"),
        ];

        let package_meta = PackageMeta::from_iter(sample_meta.into_iter());

        assert_eq!(package_meta.name, "Sample_Package");
        assert_eq!(package_meta.version, "v0.0.1");
    }

    #[test]
    #[should_panic(expected = "called `Option::unwrap()` on a `None` value")]
    fn package_meta_from_iter_fail() {
        let sample_meta = [
            String::from("package: some-package"),
            String::from("Name: Sample_Package"),
            String::from("Developed by me"),
        ];

        PackageMeta::from_iter(sample_meta.into_iter());
    }
}
