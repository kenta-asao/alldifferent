// クイーングラフ彩色問題 (Queen graph coloring problem) の問題インスタンスを生成する。
//
// 制約モデルは以下の論文の 5.1 節に基づく:
// 大野周亮, 番原睦則, 宋剛秀, 田村直之.
// "alldifferent 制約のブール基数制約への符号化手法の提案とクイーングラフ彩色問題への応用"
//
// 出力フォーマットは problem/ 以下の既存ファイル (domain/alldifferent 形式) に合わせている。
use std::env;
use std::fs::{self, File};
use std::io::{self, Write, BufWriter};

/// 位置 (i, j) (1-indexed) に対応する変数番号 x_ij を返す。
fn var_id(i: usize, j: usize, n: usize) -> usize {
    (i - 1) * n + j
}

fn write_alldifferent(writer: &mut impl Write, vars: &[usize]) -> io::Result<()> {
    write!(writer, "alldifferent")?;
    for &v in vars {
        write!(writer, " {}", v)?;
    }
    writeln!(writer, " 0")
}

fn generate_queen(n: usize, writer: &mut impl Write) -> io::Result<()> {
    writeln!(writer, "# queens graph coloring problem (n={}, colors=d={})", n, n)?;
    writeln!(writer, "domain {}", n)?;

    // 対称解除去: 1 行目 j 列目に配置されるクイーンの色を j に固定する (x_1j = j)。
    // これがないと UNSAT インスタンスの探索が N! 通りの対称解を含み桁違いに遅くなる。
    for j in 1..=n {
        writeln!(writer, "domain {} {} 0", var_id(1, j, n), j)?;
    }

    // 各行 i: alldiff { x_ij | j in N }
    for i in 1..=n {
        let vars: Vec<usize> = (1..=n).map(|j| var_id(i, j, n)).collect();
        write_alldifferent(writer, &vars)?;
    }

    // 各列 j: alldiff { x_ij | i in N }
    for j in 1..=n {
        let vars: Vec<usize> = (1..=n).map(|i| var_id(i, j, n)).collect();
        write_alldifferent(writer, &vars)?;
    }

    // 各右下がり対角線 (i - j = d, d in D = {2-n, ..., n-2}): alldiff { x_ij | i - j = d }
    for d in -(n as isize - 2)..=(n as isize - 2) {
        let vars: Vec<usize> = (1..=n)
            .filter_map(|i| {
                let j = i as isize - d;
                (j >= 1 && j <= n as isize).then(|| var_id(i, j as usize, n))
            })
            .collect();
        write_alldifferent(writer, &vars)?;
    }

    // 各右上がり対角線 (i + j = u, u in U = {3, ..., 2n-1}): alldiff { x_ij | i + j = u }
    for u in 3..=(2 * n - 1) {
        let vars: Vec<usize> = (1..=n)
            .filter_map(|i| {
                let j = u as isize - i as isize;
                (j >= 1 && j <= n as isize).then(|| var_id(i, j as usize, n))
            })
            .collect();
        write_alldifferent(writer, &vars)?;
    }

    writeln!(writer, "# --- Constraints End ---")?;
    Ok(())
}

fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    // 引数で N を1つ以上指定できる (例: cargo run -- 9 12)。
    // 指定がなければ論文のベンチマーク範囲 N=5..=12 を生成する。
    let ns: Vec<usize> = if args.len() > 1 {
        args[1..]
            .iter()
            .map(|s| {
                s.parse().unwrap_or_else(|_| {
                    eprintln!("Error: invalid N '{}'", s);
                    std::process::exit(1);
                })
            })
            .collect()
    } else {
        (5..=12).collect()
    };

    let output_dir = "/home/kenta/git/cpaior-asao/problem";
    fs::create_dir_all(output_dir)?;

    for n in ns {
        let path = format!("{}/queen_{}.txt", output_dir, n);
        let file = File::create(&path)?;
        let mut writer = BufWriter::new(file);
        generate_queen(n, &mut writer)?;
        writer.flush()?;
        println!("Generated {}", path);
    }

    Ok(())
}
