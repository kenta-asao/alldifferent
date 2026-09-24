// パンラテン方格 (Pan-Latin square / pandiagonal Latin square) 問題の問題インスタンスを生成する。
//
// 各セル (r, c) に 1 つの変数 x_rc を割り当て、その値域を 1..n とする。
// 各行・各列に加えて、巡回する主対角線・反対角線 (broken diagonals) もそれぞれ
// alldifferent 制約を満たすことを要求する (パンラテン方格の定義。単なるラテン方格には
// 対角線制約がないため常に SAT になるが、パンラテン方格は gcd(n, 6) != 1 のとき UNSAT になる)。
//
// 出力フォーマットは queen/ と同じ domain/alldifferent 形式に合わせている。
use std::env;
use std::fs::{self, File};
use std::io::{self, Write, BufWriter};

/// 位置 (r, c) (1-indexed) に対応する変数番号 x_rc を返す。
fn var_id(r: usize, c: usize, n: usize) -> usize {
    (r - 1) * n + c
}

fn write_alldifferent(writer: &mut impl Write, vars: &[usize]) -> io::Result<()> {
    write!(writer, "alldifferent")?;
    for &v in vars {
        write!(writer, " {}", v)?;
    }
    writeln!(writer, " 0")
}

fn generate_pan_latin(n: usize, writer: &mut impl Write) -> io::Result<()> {
    writeln!(writer, "# pan-Latin square problem (n={})", n)?;
    writeln!(writer, "domain {}", n)?;

    // 各行 r: alldiff { x_rc | c in N }
    for r in 1..=n {
        let vars: Vec<usize> = (1..=n).map(|c| var_id(r, c, n)).collect();
        write_alldifferent(writer, &vars)?;
    }

    // 各列 c: alldiff { x_rc | r in N }
    for c in 1..=n {
        let vars: Vec<usize> = (1..=n).map(|r| var_id(r, c, n)).collect();
        write_alldifferent(writer, &vars)?;
    }

    // 各主対角線 (d in 0..n, c = (d + r) mod n の巡回対角線): alldiff { x_rc | (c - r) ≡ d (mod n) }
    for d in 0..n {
        let vars: Vec<usize> = (0..n)
            .map(|r| {
                let c = (d + r) % n;
                var_id(r + 1, c + 1, n)
            })
            .collect();
        write_alldifferent(writer, &vars)?;
    }

    // 各反対角線 (d in 0..n, c = (d + n - r) mod n の巡回対角線): alldiff { x_rc | (c + r) ≡ d (mod n) }
    for d in 0..n {
        let vars: Vec<usize> = (0..n)
            .map(|r| {
                let c = (d + n - r) % n;
                var_id(r + 1, c + 1, n)
            })
            .collect();
        write_alldifferent(writer, &vars)?;
    }

    writeln!(writer, "# --- Constraints End ---")?;
    Ok(())
}

/// メインの処理
fn main() -> io::Result<()> {
    // --- コマンドライン引数の処理 ---
    // 引数なし: デフォルト範囲 N=5..=12 を生成する。
    // 引数2つ <start> <end>: N=start..=end (両端含む) を生成する。
    let args: Vec<String> = env::args().collect();

    let ns: Vec<usize> = match args.len() {
        1 => (5..=12).collect(),
        3 => {
            let start: usize = args[1].parse().unwrap_or_else(|_| {
                eprintln!("Error: invalid start '{}'", args[1]);
                std::process::exit(1);
            });
            let end: usize = args[2].parse().unwrap_or_else(|_| {
                eprintln!("Error: invalid end '{}'", args[2]);
                std::process::exit(1);
            });
            if start > end {
                eprintln!("Error: start ({}) must be <= end ({})", start, end);
                std::process::exit(1);
            }
            (start..=end).collect()
        }
        _ => {
            eprintln!("Usage: {} [<start> <end>]", &args[0]);
            eprintln!("Generates Pan-Latin square instances for N in [start, end] (inclusive).");
            eprintln!("With no arguments, generates N=5..=12.");
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "Invalid arguments"));
        }
    };

    // --- ファイルの準備 ---
    let output_dir = "/home/kenta/git/alldifferent/problem";
    fs::create_dir_all(output_dir)?;

    for n in ns {
        let path = format!("{}/pan_latin_{}.txt", output_dir, n);
        let file = File::create(&path)?;
        let mut writer = BufWriter::new(file);
        generate_pan_latin(n, &mut writer)?;
        writer.flush()?;
        println!("Generated {}", path);
    }

    Ok(())
}