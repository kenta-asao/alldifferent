/// clasp を全解列挙モード(-n 0)で実行し，標準出力をそのまま返す。
pub fn run_all_models(cnf_path: &str) -> String {
    let output = std::process::Command::new("clasp")
        .arg("-n")
        .arg("0")
        .arg(cnf_path)
        .output()
        .unwrap_or_else(|e| {
            panic!(
                "clasp の実行に失敗しました({}): PATH に clasp があるか確認してください",
                e
            )
        });
    String::from_utf8_lossy(&output.stdout).into_owned()
}

/// clasp の標準出力から "v ..." で始まる行(1つの解を表すリテラル列)を
/// すべて読み取る。1つの解が複数行にまたがる場合(長い解)も，
/// 終端の 0 が現れるまで連結して1つの解として扱う。
pub fn parse_solutions(raw: &str) -> Vec<Vec<i32>> {
    let mut solutions = Vec::new();
    let mut current: Vec<i32> = Vec::new();

    for line in raw.lines() {
        if let Some(rest) = line.strip_prefix("v ") {
            for tok in rest.split_whitespace() {
                if let Ok(lit) = tok.parse::<i32>() {
                    if lit == 0 {
                        solutions.push(std::mem::take(&mut current));
                    } else {
                        current.push(lit);
                    }
                }
            }
        }
    }

    solutions
}
