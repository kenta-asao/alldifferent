mod cadical;
mod clasp;
mod cnf;
mod decode;
mod encoding;
mod parser;

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process;

// n_base を基準に，これまでに割り当てた補助変数の個数 m を 1 増やし，新しい変数番号を発行する。
// encoding.rs の at_most_k などが用いる "変数番号 = n_base + m" という規約に合わせるためのヘルパー。
fn fresh_var(n_base: i32, m: &mut i32) -> i32 {
    *m += 1;
    n_base + *m
}

// Bessiere (IJCAI 2009) の実験 (HI_k) にならい，range consistency 分解を課す
// Hall 区間のサイズ k = u - l + 1 を制限するためのフィルタ。
// n はその alldifferent 制約の変数の個数 (区間サイズは高々 n)。
// unused value propagation 分解には影響しない。
enum RangeFilter {
    // 制限なし (デフォルト): 全てのサイズの区間に分解を課す
    All,
    // --only N: サイズがちょうど N の区間だけに分解を課す
    Only(i32),
    // --under N: サイズが N 以下の区間だけに分解を課す (論文の HI_k に対応)
    Under(i32),
    // --max: 各 alldifferent 制約ごとに，意味のある最大サイズ (n - 1) の区間だけに分解を課す
    Max,
}

impl RangeFilter {
    fn allows(&self, k: i32, n: i32) -> bool {
        // サイズ1の Hall 区間 (「各値はちょうど1つの変数に割り当てられる」) は，
        // range consistency 分解単体で見れば alldifferent 制約そのものを成立させる
        // 基本制約にあたる (range_consistency クレートではこれが欠けると本来 UNSAT な
        // 問題が誤って SAT と判定され得る)。このクレードでは unused value propagation
        // 分解が別途 alldifferent の正しさを保証するため健全性への影響はないが，
        // --only/--under/--max の意味を range_consistency クレートと揃えるために
        // 同じ規則 (サイズ1は常に含める) を適用する。
        if k == 1 {
            return true;
        }
        match self {
            RangeFilter::All => true,
            RangeFilter::Only(size) => k == *size,
            RangeFilter::Under(size) => k <= *size,
            RangeFilter::Max => k == n - 1,
        }
    }

    // 出力ファイル名に付与するサフィックス (例: "_under3")。
    // 条件ごとに出力ファイルを分けて，異なる条件での実行結果が上書きされないようにする。
    // フィルタなし (All) の場合は空文字列 (従来通りのファイル名のまま)。
    fn suffix(&self) -> String {
        match self {
            RangeFilter::All => String::new(),
            RangeFilter::Only(size) => format!("_only{}", size),
            RangeFilter::Under(size) => format!("_under{}", size),
            RangeFilter::Max => "_max".to_string(),
        }
    }
}

// args (プログラム名・入力ファイルパスを除いたフラグ部分) から --only/--under/--max を読み取る。
// 3つは互いに排他 (同時に2つ以上指定するとエラー)。値を取る --only/--under は，フラグの次の
// トークンを i32 としてパースする (欠けている・数値でない場合はエラー)。
fn parse_range_filter(args: &[String]) -> RangeFilter {
    let parse_value = |flag: &str| -> Option<i32> {
        args.iter().position(|a| a == flag).map(|i| {
            let raw = args.get(i + 1).unwrap_or_else(|| {
                eprintln!("エラー: {} には数値を指定してください。", flag);
                process::exit(1);
            });
            raw.parse::<i32>().unwrap_or_else(|_| {
                eprintln!("エラー: {} の値 '{}' は数値ではありません。", flag, raw);
                process::exit(1);
            })
        })
    };

    let only = parse_value("--only");
    let under = parse_value("--under");
    let max = args.iter().any(|a| a == "--max");

    let specified_count = [only.is_some(), under.is_some(), max]
        .iter()
        .filter(|&&b| b)
        .count();
    if specified_count > 1 {
        eprintln!("エラー: --only / --under / --max は同時に指定できません。どれか1つを選んでください。");
        process::exit(1);
    }

    if let Some(n) = only {
        RangeFilter::Only(n)
    } else if let Some(n) = under {
        RangeFilter::Under(n)
    } else if max {
        RangeFilter::Max
    } else {
        RangeFilter::All
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "Usage: {} <folder>/<problem_file> [--decode | --queen] [--clasp | --cadical] [--all] [--server] [--only N | --under N | --max]",
            args[0]
        );
        process::exit(1);
    }
    let input_path = &args[1];
    // --queen: クイーングラフ彩色問題専用のデコード形式 (盤面表示) を使う。指定すると自動的に --decode も有効になる。
    // --decode か --queen のどちらか一方を指定すればよい (両方指定した場合は --queen を優先する)。
    let use_queen = args[2..].iter().any(|a| a == "--queen");
    let do_decode = use_queen || args[2..].iter().any(|a| a == "--decode");
    // --all: clasp を -n 0 (全解列挙) で実行する。指定しない場合は -n 1 (単解探索)。
    let enumerate_all = args[2..].iter().any(|a| a == "--all");
    // ソルバーは --clasp か --cadical のどちらか一方で指定する (どちらも指定しない場合は clasp を使う)。
    let use_clasp = args[2..].iter().any(|a| a == "--clasp");
    let use_cadical = args[2..].iter().any(|a| a == "--cadical");
    if use_clasp && use_cadical {
        eprintln!("エラー: --clasp と --cadical は同時に指定できません。どちらか一方を選んでください。");
        process::exit(1);
    }
    // --server: --cadical と併用時，ローカルビルドではなくサーバー上のバイナリを使う。
    let use_server = args[2..].iter().any(|a| a == "--server");
    // --only/--under/--max: range consistency 分解を課す Hall 区間のサイズを制限する。
    let range_filter = parse_range_filter(&args[2..]);

    let problem = parser::parse_file(input_path).unwrap_or_else(|e| {
        eprintln!("入力ファイルの解析に失敗しました: {}", e);
        process::exit(1);
    });

    // 入力パスがどんなフォルダ階層にあっても，出力はフォルダ名を持たず
    // "<問題名>.拡張子" だけにして cnf/, result/, decode/ 直下に置く。
    // --only/--under/--max 指定時は "<問題名>_under3.拡張子" のようにサフィックスを付け，
    // 条件違いの実行結果が同じファイルに上書きされないようにする。
    let stem = Path::new(input_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");
    let stem = format!("{}{}", stem, range_filter.suffix());

    // CNF は cnf/，求解結果は result/，復号結果は decode/ フォルダに出力する。
    for dir in ["cnf", "result", "decode"] {
        if let Err(e) = fs::create_dir_all(dir) {
            eprintln!("フォルダの作成に失敗しました ({}): {}", dir, e);
            process::exit(1);
        }
    }
    let cnf_path = format!("cnf/rcuvp_{}.cnf", stem);
    let result_path = format!("result/rcuvp_result_{}.txt", stem);
    let decode_path = format!("decode/rcuvp_decode_{}.txt", stem);

    let d = problem.d;

    // 制約に登場する変数 (domain 宣言 or alldifferent) を全て集める。
    let mut var_ids: Vec<i32> = Vec::new();
    for vars in &problem.alldiffs {
        for &v in vars {
            if !var_ids.contains(&v) {
                var_ids.push(v);
            }
        }
    }
    for &v in problem.domains.keys() {
        if !var_ids.contains(&v) {
            var_ids.push(v);
        }
    }
    var_ids.sort();

    // p_ij (x_i = j) 変数を全変数・全値域に対して割り当てる。
    let mut pij: HashMap<(i32, i32), i32> = HashMap::new();
    let mut next_var = 0;
    for &v in &var_ids {
        for j in 1..=d {
            next_var += 1;
            pij.insert((v, j), next_var);
        }
    }
    let n_base = next_var;
    let mut m: i32 = 0;

    let mut cnf = cnf::Cnf::new();

    // 各変数はちょうど1つの値をとる (制約ごとの重複を避けるため変数ごとに1度だけ追加)。
    for &v in &var_ids {
        let lits: Vec<i32> = (1..=d).map(|j| pij[&(v, j)]).collect();
        let (clauses, m1) = encoding::exact_one(lits, n_base, m);
        cnf.add_clauses(clauses);
        m = m1;
    }

    // domain 宣言による値域制限 (指定された値以外を禁止する単位節)。
    for (&v, allowed) in &problem.domains {
        for j in 1..=d {
            if !allowed.contains(&j) {
                cnf.add_clause(vec![-pij[&(v, j)]]);
            }
        }
    }

    // 各 alldifferent 制約を，range consistency 分解 (Bessiere, IJCAI 2009) と
    // unused value propagation 分解 (uv_propagation の提案手法4) の両方で符号化する。
    // 同じ p_ij を土台に，異なる伝播原理に基づく制約を冗長に重ねがけすることで，
    // 単独では検出しづらい枝刈りをどちらか一方の分解が補うことを狙う。
    for vars in &problem.alldiffs {
        let n = vars.len() as i32;

        // --- range consistency 分解 ---
        // 1 <= l <= u <= d かつ u - l < n を満たす各区間 [l, u] ごとに，
        // 変数 X_i ごとの補助変数 A_{i,l,u} <=> X_i ∈ [l, u] を導入し，
        // sum_i A_{i,l,u} <= u - l + 1 (区間の大きさを超える個数の変数を割り当てられない) を課す。
        // u - l + 1 == n の場合は変数の総数から自明に成り立つため省略する。
        for l in 1..=d {
            for u in l..=d {
                if u - l >= n {
                    continue;
                }
                let k = u - l + 1;
                if !range_filter.allows(k, n) {
                    continue;
                }

                let mut a_vars: Vec<i32> = Vec::new();
                for &i in vars {
                    let a = fresh_var(n_base, &mut m);
                    a_vars.push(a);

                    let range_lits: Vec<i32> = (l..=u).map(|j| pij[&(i, j)]).collect();

                    // A_{i,l,u} => X_i ∈ [l, u]
                    let mut clause = vec![-a];
                    clause.extend(range_lits.iter().copied());
                    cnf.add_clause(clause);

                    // X_i ∈ [l, u] => A_{i,l,u}
                    for &lit in &range_lits {
                        cnf.add_clause(vec![-lit, a]);
                    }

                    // (1) の逆方向: X_i ∉ [l, u] => ¬A_{i,l,u} を，区間外のリテラルの論理和として明示する。
                    // 上の2つの節群だけでは，区間外の値が「1つを除いて全て偽」になるまで単位伝播が
                    // 発火せず，区間の候補が2つ以上残っている間は A が確定しない (range_consistency
                    // クレートと同じ理由，詳細はそちらのコメント参照)。この節を加えることで，区間外が
                    // 全て偽になった時点で単位伝播だけで A=1 を導けるようにする (論理的には既存の
                    // 制約から導かれる冗長節)。
                    let mut outside_clause = vec![a];
                    for j in 1..=d {
                        if j < l || j > u {
                            outside_clause.push(pij[&(i, j)]);
                        }
                    }
                    cnf.add_clause(outside_clause);
                }

                if k < n {
                    let (clauses, m1) = encoding::at_most_k(a_vars, n_base, m, k);
                    cnf.add_clauses(clauses);
                    m = m1;
                }
            }
        }

        // --- unused value propagation 分解 ---
        if n >= d {
            // n >= d の場合: 各値 j はちょうど1つの変数に割り当てられる。
            for j in 1..=d {
                let lits: Vec<i32> = vars.iter().map(|&v| pij[&(v, j)]).collect();
                let (clauses, m1) = encoding::exact_one(lits, n_base, m);
                cnf.add_clauses(clauses);
                m = m1;
            }
        } else {
            // n < d の場合: 未使用の値を吸収するダミー行を追加する。
            let mut q: HashMap<i32, i32> = HashMap::new();
            for j in 1..=d {
                q.insert(j, fresh_var(n_base, &mut m));
            }

            // ダミー行はちょうど d-n 個の値をとる (= d-n 個の値が未使用になる)。
            let dummy_lits: Vec<i32> = (1..=d).map(|j| q[&j]).collect();
            let (clauses, m1) = encoding::exact_k(dummy_lits, n_base, m, d - n);
            cnf.add_clauses(clauses);
            m = m1;

            // 各値 j は，n個の実変数とダミー行を合わせてちょうど1つに割り当てられる。
            for j in 1..=d {
                let mut lits: Vec<i32> = vars.iter().map(|&v| pij[&(v, j)]).collect();
                lits.push(q[&j]);
                let (clauses, m1) = encoding::exact_one(lits, n_base, m);
                cnf.add_clauses(clauses);
                m = m1;
            }
        }
    }

    cnf.num_vars = n_base + m;

    if let Err(e) = cnf.write_to_file(&cnf_path) {
        eprintln!("CNF ファイルの書き込みに失敗しました: {}", e);
        process::exit(1);
    }
    println!("CNF を書き出しました: {}", cnf_path);

    // 求解やデコードを要求するフラグ (--clasp/--cadical/--all/--decode/--queen) が1つも
    // 指定されていない場合 (--only/--under/--max/--server だけの指定を含む) は，
    // 符号化して CNF を書き出すところまでで終了する。
    if !use_clasp && !use_cadical && !enumerate_all && !do_decode {
        println!("求解・デコードのフラグが指定されていないため，CNF の書き出しまでで終了します。");
        return;
    }

    if use_cadical {
        if enumerate_all {
            eprintln!("警告: --all は cadical では無視されます (cadical は単一解探索のみ対応です)。");
        }
        if let Err(e) = cadical::run_cadical(&cnf_path, &result_path, use_server) {
            eprintln!("cadical の実行に失敗しました: {}", e);
            process::exit(1);
        }
        println!("cadical の実行結果を書き出しました: {}", result_path);
    } else {
        if let Err(e) = clasp::run_clasp(&cnf_path, &result_path, enumerate_all) {
            eprintln!("clasp の実行に失敗しました: {}", e);
            process::exit(1);
        }
        println!("clasp の実行結果を書き出しました: {}", result_path);
    }

    if do_decode {
        let decode_result = if use_queen {
            decode::decode_queen(&result_path, &decode_path, &pij, &var_ids, d)
        } else {
            decode::decode(&result_path, &decode_path, &pij, &var_ids, d)
        };
        if let Err(e) = decode_result {
            eprintln!("復号化に失敗しました: {}", e);
            process::exit(1);
        }
        println!("復号結果を書き出しました: {}", decode_path);
    }
}
