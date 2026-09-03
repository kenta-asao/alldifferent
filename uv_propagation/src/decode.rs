use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::io;

enum SolveOutcome {
    Unsat,
    Unknown,
    Sat(Vec<HashSet<i32>>),
}

// clasp/cadical の実行結果 (result_path) を読み込み，充足可否と各解の正リテラル集合を取り出す。
// 解 (Answer) ごとに "v " で始まる行 (DIMACS SAT 出力形式) の正リテラルをまとめる。
// clasp は各解の直前に "c Answer: N" という行を出すため，これを区切りとして解を分割する
// (-n 0 による全解列挙の場合は複数の Answer ブロックが並ぶ)。
// cadical のように "c Answer:" 行を出さない (常に単一解の) ソルバーの場合は，
// 最初の "v " 行に出会った時点で暗黙に 1 つ目の解ブロックを開始する。
fn read_outcome(result_path: &str) -> io::Result<SolveOutcome> {
    let content = fs::read_to_string(result_path)?;
    let lines: Vec<&str> = content.lines().collect();

    // clasp/cadical の状態行は "s SATISFIABLE" / "s UNSATISFIABLE" ("s " が付かない場合もある)。
    let is_status = |l: &str, status: &str| {
        let t = l.trim();
        t == status || t == format!("s {}", status)
    };

    if lines.iter().any(|l| is_status(l, "UNSATISFIABLE")) {
        return Ok(SolveOutcome::Unsat);
    }
    if !lines.iter().any(|l| is_status(l, "SATISFIABLE")) {
        return Ok(SolveOutcome::Unknown);
    }

    let mut answers: Vec<HashSet<i32>> = Vec::new();
    let mut current: Option<HashSet<i32>> = None;
    for line in &lines {
        let trimmed = line.trim();
        let trimmed = trimmed.strip_prefix("c ").unwrap_or(trimmed);
        if trimmed.starts_with("Answer:") {
            if let Some(prev) = current.take() {
                answers.push(prev);
            }
            current = Some(HashSet::new());
            continue;
        }
        if let Some(rest) = line.trim().strip_prefix("v ") {
            if current.is_none() {
                current = Some(HashSet::new());
            }
            if let Some(set) = current.as_mut() {
                for tok in rest.split_whitespace() {
                    if let Ok(lit) = tok.parse::<i32>() {
                        if lit > 0 {
                            set.insert(lit);
                        }
                    }
                }
            }
        }
    }
    if let Some(prev) = current.take() {
        answers.push(prev);
    }

    Ok(SolveOutcome::Sat(answers))
}

// p_ij (satvar) -> (x_i, j) の逆引きマップを構築する。
fn build_reverse(pij: &HashMap<(i32, i32), i32>) -> HashMap<i32, (i32, i32)> {
    let mut reverse: HashMap<i32, (i32, i32)> = HashMap::new();
    for (&(var, j), &satvar) in pij.iter() {
        reverse.insert(satvar, (var, j));
    }
    reverse
}

// clasp/cadical の実行結果 (result_path) を読み込み，p_ij 変数の割り当てから
// 元の alldifferent 変数 x_i = j の割り当てへ復号する。
// 復号結果は標準出力には表示せず，decode_path にのみ書き出す。
pub fn decode(
    result_path: &str,
    decode_path: &str,
    pij: &HashMap<(i32, i32), i32>,
    var_ids: &[i32],
    d: i32,
) -> io::Result<()> {
    let answers = match read_outcome(result_path)? {
        SolveOutcome::Unsat => {
            fs::write(decode_path, "UNSATISFIABLE\n")?;
            return Ok(());
        }
        SolveOutcome::Unknown => {
            fs::write(
                decode_path,
                "求解結果を判定できませんでした。result ファイルの内容を確認してください。\n",
            )?;
            return Ok(());
        }
        SolveOutcome::Sat(answers) => answers,
    };

    let reverse = build_reverse(pij);

    let mut output = String::new();
    writeln!(output, "SATISFIABLE").unwrap();
    let multiple = answers.len() > 1;
    for (i, positive) in answers.iter().enumerate() {
        let mut assignment: HashMap<i32, i32> = HashMap::new();
        for &lit in positive {
            if let Some(&(var, j)) = reverse.get(&lit) {
                assignment.insert(var, j);
            }
        }

        if multiple {
            writeln!(output, "Answer {}:", i + 1).unwrap();
        }
        for &var in var_ids {
            match assignment.get(&var) {
                Some(&j) => writeln!(output, "x{} = {}", var, j).unwrap(),
                None => writeln!(output, "x{} = ? (未割り当て, d={})", var, d).unwrap(),
            }
        }
    }
    if multiple {
        writeln!(output, "Models: {}", answers.len()).unwrap();
    }

    fs::write(decode_path, output)?;

    Ok(())
}

// クイーングラフ彩色問題専用の復号。
// セル変数は行優先 ((r-1)*n+c) の順に並んでいる前提で，var_ids をソート済みの順番のまま
// n×n の盤面 (行ごとに色番号を並べたもの) として出力する。n は変数の個数の平方根から求める。
pub fn decode_queen(
    result_path: &str,
    decode_path: &str,
    pij: &HashMap<(i32, i32), i32>,
    var_ids: &[i32],
    d: i32,
) -> io::Result<()> {
    let answers = match read_outcome(result_path)? {
        SolveOutcome::Unsat => {
            fs::write(decode_path, "UNSATISFIABLE\n")?;
            return Ok(());
        }
        SolveOutcome::Unknown => {
            fs::write(
                decode_path,
                "求解結果を判定できませんでした。result ファイルの内容を確認してください。\n",
            )?;
            return Ok(());
        }
        SolveOutcome::Sat(answers) => answers,
    };

    let num_cells = var_ids.len();
    let n = (num_cells as f64).sqrt().round() as usize;
    if n * n != num_cells {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "変数の個数 ({}) が平方数ではないため，クイーングラフの盤面として復号できません。",
                num_cells
            ),
        ));
    }

    let reverse = build_reverse(pij);
    let width = d.to_string().len() + 1;

    let mut output = String::new();
    writeln!(output, "SATISFIABLE").unwrap();
    let multiple = answers.len() > 1;
    for (ans_idx, positive) in answers.iter().enumerate() {
        let mut assignment: HashMap<i32, i32> = HashMap::new();
        for &lit in positive {
            if let Some(&(var, j)) = reverse.get(&lit) {
                assignment.insert(var, j);
            }
        }

        if multiple {
            writeln!(output, "Answer {}:", ans_idx + 1).unwrap();
        }
        for r in 0..n {
            for c in 0..n {
                let var = var_ids[r * n + c];
                match assignment.get(&var) {
                    Some(&j) => write!(output, "{:>width$}", j, width = width).unwrap(),
                    None => write!(output, "{:>width$}", "?", width = width).unwrap(),
                }
            }
            writeln!(output).unwrap();
        }
    }
    if multiple {
        writeln!(output, "Models: {}", answers.len()).unwrap();
    }

    fs::write(decode_path, output)?;

    Ok(())
}
