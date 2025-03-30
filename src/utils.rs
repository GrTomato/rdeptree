use std::fs::File;
use std::io::{self, BufRead};
use std::path::Path;

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
