use std::fs::File;
use std::io;
use std::process::{Command, Stdio};

const LOCAL_CADICAL_PATH: &str = "/home/kenta/git/programing/cadical/build/cadical";
const SERVER_CADICAL_PATH: &str = "/home/share/app/cadical";

// cnf_path を cadical で求解し，標準出力を result_path に書き出す。
// cadical は単一解探索のみ対応 (clasp の -n 0 のような全解列挙はできない)。
// use_server が true の場合はサーバー上のバイナリ，false の場合はローカルビルドを使う。
pub fn run_cadical(cnf_path: &str, result_path: &str, use_server: bool) -> io::Result<()> {
    let output_file = File::create(result_path)?;
    let cadical_path = if use_server {
        SERVER_CADICAL_PATH
    } else {
        LOCAL_CADICAL_PATH
    };

    Command::new(cadical_path)
        .arg(cnf_path)
        .stdout(Stdio::from(output_file))
        .status()?;

    Ok(())
}
