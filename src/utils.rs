use std::fs;
use std::fs::DirEntry;
use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;
use std::path::PathBuf;

const METADATA_DIR_SUFFIX: &'static str = ".dist-info";

/// from https://doc.rust-lang.org/rust-by-example/std_misc/file/read_lines.html
pub fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

pub fn get_lnreader<P, F>(
    filename: P,
    stop_func: F,
) -> Result<impl Iterator<Item = String>, io::Error>
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
pub fn get_meta_dirs(env_path: &PathBuf) -> impl Iterator<Item = DirEntry> {
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
