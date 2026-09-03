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

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!(
            "Usage: {} <folder>/<problem_file> [--decode | --queen] [--clasp | --cadical] [--all] [--server]",
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

    let problem = parser::parse_file(input_path).unwrap_or_else(|e| {
        eprintln!("入力ファイルの解析に失敗しました: {}", e);
        process::exit(1);
    });

    // 入力パスがどんなフォルダ階層にあっても，出力はフォルダ名を持たず
    // "<問題名>.拡張子" だけにして cnf/, result/, decode/ 直下に置く。
    let stem = Path::new(input_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("output");

    // CNF は cnf/，求解結果は result/，復号結果は decode/ フォルダに出力する。
    for dir in ["cnf", "result", "decode"] {
        if let Err(e) = fs::create_dir_all(dir) {
            eprintln!("フォルダの作成に失敗しました ({}): {}", dir, e);
            process::exit(1);
        }
    }
    let cnf_path = format!("cnf/rc_{}.cnf", stem);
    let result_path = format!("result/rc_result_{}.txt", stem);
    let decode_path = format!("decode/rc_decode_{}.txt", stem);

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

    // 各 alldifferent 制約を Bessiere (IJCAI 2009) の range consistency 分解で符号化する。
    //
    // AllDifferent({X_i}) (|{X_i}| = n) に対し，1 <= l <= u <= d かつ u - l < n を満たす
    // 各区間 [l, u] ごとに，制約内の変数 X_i ごとの補助変数 A_{i,l,u} を導入し，以下を課す。
    //   (1) A_{i,l,u} <=> X_i ∈ [l, u]   (⇔ OR_{j=l}^{u} p_ij ，p_ij は exact-one 済みなので同値となる)
    //   (2) sum_i A_{i,l,u} <= u - l + 1  (区間 [l, u] にはその大きさ以上の変数を割り当てられない)
    // u - l + 1 == n の場合，(2) は変数の総数がちょうど n 個であることから自明に成り立つため省略する。
    for vars in &problem.alldiffs {
        let n = vars.len() as i32;

        for l in 1..=d {
            for u in l..=d {
                if u - l >= n {
                    continue;
                }

                let mut a_vars: Vec<i32> = Vec::new();
                for &i in vars {
                    let a = fresh_var(n_base, &mut m);
                    a_vars.push(a);

                    let range_lits: Vec<i32> = (l..=u).map(|j| pij[&(i, j)]).collect();

                    // (1) A_{i,l,u} => X_i ∈ [l, u] (すなわち range_lits のいずれかが真)
                    let mut clause = vec![-a];
                    clause.extend(range_lits.iter().copied());
                    cnf.add_clause(clause);

                    // (1) X_i ∈ [l, u] => A_{i,l,u} (range_lits の各リテラルから A_{i,l,u} を導出)
                    for &lit in &range_lits {
                        cnf.add_clause(vec![-lit, a]);
                    }
                }

                // (2) sum_i A_{i,l,u} <= u - l + 1
                let k = u - l + 1;
                if k < n {
                    let (clauses, m1) = encoding::at_most_k(a_vars, n_base, m, k);
                    cnf.add_clauses(clauses);
                    m = m1;
                }
            }
        }
    }

    cnf.num_vars = n_base + m;

    if let Err(e) = cnf.write_to_file(&cnf_path) {
        eprintln!("CNF ファイルの書き込みに失敗しました: {}", e);
        process::exit(1);
    }
    println!("CNF を書き出しました: {}", cnf_path);

    // 問題ファイルのパスだけを指定した場合 (ソルバーやデコードに関するフラグが1つもない場合) は，
    // 符号化して CNF を書き出すところまでで終了する。
    if args.len() <= 2 {
        println!("フラグが指定されていないため，CNF の書き出しまでで終了します。");
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
