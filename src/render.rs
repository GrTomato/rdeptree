use crate::DistrMeta;

/// Print results of the program, i.e. the list of installed
/// packages and interpreter path
pub fn render_output(packages: &Vec<DistrMeta>) {
    // println!("For Python env in {}", interpreter_loc);
    println!("Installed packages:");
    for p in packages {
        println!("{}", p);
    }
}
