use std::fs::File;
use std::io;
use std::process::{Command, Stdio};

// cnf_path を clasp で求解し，標準出力を result_path に書き出す。
// enumerate_all が true の場合は "-n 0" (全解列挙)，false の場合は "-n 1" (単解探索) を用いる。
pub fn run_clasp(cnf_path: &str, result_path: &str, enumerate_all: bool) -> io::Result<()> {
    let output_file = File::create(result_path)?;
    let n_arg = if enumerate_all { "0" } else { "1" };

    Command::new("clasp")
        .args(&["-n", n_arg, cnf_path])
        .stdout(Stdio::from(output_file))
        .status()?;

    Ok(())
}
