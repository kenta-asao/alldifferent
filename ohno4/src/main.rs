mod clasp;
mod dyscription;
mod encoding;

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::Path;

/// 未使用の変数番号を順番に払い出すだけの単純なプール。
/// encoding::at_most_k は補助変数の開始番号として `n + m` を使うため，
/// 常に n = pool.total(), m = 0 で呼び出し，返ってきた m の分だけ
/// プールを進めることで，重複のない連番を保証する。
struct VarPool {
    next: i32,
}

impl VarPool {
    fn new() -> Self {
        VarPool { next: 1 }
    }

    fn alloc(&mut self, count: i32) -> Vec<i32> {
        let vars: Vec<i32> = (self.next..self.next + count).collect();
        self.next += count;
        vars
    }

    fn total(&self) -> i32 {
        self.next - 1
    }

    fn advance(&mut self, delta: i32) {
        self.next += delta;
    }
}

/// x_1 + ... + x_n = 1 (ちょうど1つが真) を表す節を生成する．
/// (5.1),(5.3),(5.10) など，本論文中の "= 1" 制約はすべてこの形。
fn exact_one(vars: &Vec<i32>, pool: &mut VarPool) -> Vec<Vec<i32>> {
    let mut clauses = Vec::new();

    // at-least-1 : 少なくとも1つは真 (単一のOR節)
    clauses.push(encoding::at_least_one(vars));

    // at-most-1 : 高々1つが真
    let (mut c, m) = encoding::at_most_k(vars.clone(), pool.total(), 0, 1);
    pool.advance(m);
    clauses.append(&mut c);

    clauses
}

/// x_1 + ... + x_n = k (ちょうどk個が真) を表す節を生成する．
/// 提案手法4の (5.9) (ダミー行の合計が d-n になる制約) で使用する．
/// at-most-k は encoding::at_most_k をそのまま，
/// at-least-k は「否定リテラルに対する at-most-(n-k)」として実現する
/// (¬x が高々 n-k 個真 ⟺ x が少なくとも k 個真)．
fn exact_k(vars: &Vec<i32>, k: i32, pool: &mut VarPool) -> Vec<Vec<i32>> {
    let n = vars.len() as i32;
    let mut clauses = Vec::new();

    if k <= 0 {
        for &v in vars {
            clauses.push(vec![-v]);
        }
        return clauses;
    }
    if k >= n {
        for &v in vars {
            clauses.push(vec![v]);
        }
        return clauses;
    }

    // at-most-k (上限)
    let (mut c1, m1) = encoding::at_most_k(vars.clone(), pool.total(), 0, k);
    pool.advance(m1);
    clauses.append(&mut c1);

    // at-least-k => at-most-(n-k) on negated literals (下限)
    let negated: Vec<i32> = vars.iter().map(|&v| -v).collect();
    let (mut c2, m2) = encoding::at_most_k(negated, pool.total(), 0, n - k);
    pool.advance(m2);
    clauses.append(&mut c2);

    clauses
}

/// example.txt 形式を読み込む．
/// "#" で始まる行はコメントとして無視する．
///
/// "domain" で始まる行は2通りの意味を持つ:
///   - "domain <d>"                  : 明示ドメイン指定のない変数に使う
///                                      デフォルトドメイン {1,...,d} を設定する
///   - "domain <var> v1 v2 ... 0"    : 変数 <var> 専用のドメイン {v1,v2,...}
///                                      を指定する(末尾は0で終端)
/// トークン数2なら前者，3以上なら後者として扱う．
///
/// それ以外の行は "<ラベル> v1 v2 ... vn 0" の形式で，
/// 先頭のラベル(例: "alldifferent")は無視し，末尾の 0 で終端された
/// 整数列を1つの alldifferent 制約として読み取る．
fn parse_input(path: &str) -> (Vec<Vec<i32>>, Option<i32>, HashMap<i32, Vec<i32>>) {
    let content = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("入力ファイル {} を読み込めません: {}", path, e));

    let mut constraints = Vec::new();
    let mut default_domain: Option<i32> = None;
    let mut var_domains: HashMap<i32, Vec<i32>> = HashMap::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.len() < 2 {
            continue;
        }

        if tokens[0].eq_ignore_ascii_case("domain") {
            if tokens.len() == 2 {
                // "domain <d>" : デフォルトドメインサイズ
                let d: i32 = tokens[1]
                    .parse()
                    .unwrap_or_else(|_| panic!("domain の値が不正です: {}", line));
                default_domain = Some(d);
            } else {
                // "domain <var> v1 v2 ... 0" : 変数ごとのドメイン
                let var_idx: i32 = tokens[1]
                    .parse()
                    .unwrap_or_else(|_| panic!("domain の変数番号が不正です: {}", line));
                let mut values = Vec::new();
                for tok in &tokens[2..] {
                    let value: i32 = match tok.parse() {
                        Ok(v) => v,
                        Err(_) => continue,
                    };
                    if value == 0 {
                        break;
                    }
                    values.push(value);
                }
                values.sort_unstable();
                values.dedup();
                var_domains.insert(var_idx, values);
            }
            continue;
        }

        // 先頭トークン(制約の種類ラベル)は読み捨てる
        let mut vars = Vec::new();
        for tok in &tokens[1..] {
            let value: i32 = match tok.parse() {
                Ok(v) => v,
                Err(_) => continue,
            };
            if value == 0 {
                break;
            }
            vars.push(value);
        }

        if !vars.is_empty() {
            constraints.push(vars);
        }
    }

    (constraints, default_domain, var_domains)
}

/// 変数 `i` の行を，論文どおり共通の値域 `full_range` (= 1..=d) 全体に
/// わたって作成する。p_ij は j が `full_range` に含まれる限りすべて
/// 定義され((5.1)/(5.8) の exact-1 制約もこの全域に対して張る)，
/// そのうち `domain_i` に含まれない値については単位節 `¬p_ij` を追加して
/// 偽に固定する(=「その変数はこの値を取れない」をドメイン外として表現)。
fn build_row(
    i: i32,
    full_range: &Vec<i32>,
    domain_i: &Vec<i32>,
    pool: &mut VarPool,
    meaning: &mut HashMap<i32, String>,
) -> (HashMap<i32, i32>, Vec<Vec<i32>>) {
    let fresh = pool.alloc(full_range.len() as i32);
    let mut row = HashMap::new();
    for (&val, &v) in full_range.iter().zip(fresh.iter()) {
        meaning.insert(v, format!("p(x={},value={})", i, val));
        row.insert(val, v);
    }

    let mut clauses = exact_one(&fresh, pool);
    for &val in full_range {
        if !domain_i.contains(&val) {
            clauses.push(vec![-row[&val]]);
        }
    }
    (row, clauses)
}

/// 制約 `c_idx` (0-indexed) のダミー行(n+1行目)を，共通の値域
/// `full_range` (= 1..=d) 全体にわたって作成する。行自体のカーディナリ
/// ティ制約((5.9))は呼び出し側で exact_k を使って別途追加するため，
/// ここでは変数の割り当てのみ行う。
fn build_dummy_row(
    c_idx: usize,
    full_range: &Vec<i32>,
    pool: &mut VarPool,
    meaning: &mut HashMap<i32, String>,
) -> (HashMap<i32, i32>, Vec<Vec<i32>>) {
    let fresh = pool.alloc(full_range.len() as i32);
    let mut row = HashMap::new();
    for (&val, &v) in full_range.iter().zip(fresh.iter()) {
        meaning.insert(v, format!("dummy(constraint={},value={})", c_idx + 1, val));
        row.insert(val, v);
    }
    (row, Vec::new())
}

fn output_paths(input_path: &str) -> (String, String) {
    let path = Path::new(input_path);
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "output".to_string());
    let dir = path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| Path::new("").to_path_buf());

    let cnf_path = dir.join(format!("{}.cnf", stem));
    let map_path = dir.join(format!("{}.map", stem));

    (
        cnf_path.to_string_lossy().into_owned(),
        map_path.to_string_lossy().into_owned(),
    )
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let input_path = if args.len() > 1 {
        args[1].clone()
    } else {
        "example.txt".to_string()
    };

    let (constraints, default_domain, var_domains) = parse_input(&input_path);
    if constraints.is_empty() {
        panic!("入力ファイルから alldifferent 制約を読み取れませんでした: {}", input_path);
    }

    let max_var = constraints
        .iter()
        .flat_map(|c| c.iter())
        .copied()
        .max()
        .unwrap_or(0);

    // 共有される値域(論文の d に相当)。"domain <d>" があればそれを，
    // なければ example.txt の慣習(クイーングラフ彩色問題: 変数の総数 =
    // 値の総数)に従い全制約に現れる変数番号の最大値を使う。ただし
    // "domain <var> ..." で d を超える値が明示指定されていた場合は，
    // その値も含むように d を広げる(列が足りず表現できなくなるのを防ぐ)。
    let explicit_max = var_domains
        .values()
        .flat_map(|v| v.iter())
        .copied()
        .max()
        .unwrap_or(0);
    let d = default_domain.unwrap_or(max_var).max(explicit_max);
    let full_range: Vec<i32> = (1..=d).collect();

    let mut pool = VarPool::new();
    let mut clauses: Vec<Vec<i32>> = Vec::new();
    let mut meaning: HashMap<i32, String> = HashMap::new();

    // 5.1 各変数 x_i は，制約をまたいで共有される1行を1つだけ持つ。
    // 論文どおり，行は共通の値域 1..=d 全体にわたって定義され
    // ((5.8)相当のexact-1制約もこの全域に対して張る)，"domain <var> ..."
    // で個別指定されたドメインに含まれない値は単位節で偽に固定する。
    let mut used_vars: Vec<i32> = constraints.iter().flat_map(|c| c.iter().copied()).collect();
    used_vars.sort_unstable();
    used_vars.dedup();

    let mut p: HashMap<i32, HashMap<i32, i32>> = HashMap::new();
    for &i in &used_vars {
        let domain_i: Vec<i32> = match var_domains.get(&i) {
            Some(v) => v.clone(),
            None => full_range.clone(),
        };
        if domain_i.is_empty() {
            panic!("変数{}のドメインが空です", i);
        }
        let (row, row_clauses) = build_row(i, &full_range, &domain_i, &mut pool, &mut meaning);
        clauses.extend(row_clauses);
        p.insert(i, row);
    }

    // 各 alldifferent 制約を提案手法4で符号化する。
    // すべての行が共通の値域 1..=d を持つため，論文どおり d をそのまま
    // 制約の値集合として使う(n_c <= d を前提とする)。
    for (c_idx, vars_c) in constraints.iter().enumerate() {
        let n_c = vars_c.len() as i32;

        if n_c > d {
            panic!(
                "制約{}の変数数({})がdomain({})を超えています(提案手法4はn<=dを前提とします)",
                c_idx + 1,
                n_c,
                d
            );
        }

        let dummy_row: Option<HashMap<i32, i32>> = if n_c < d {
            let (row, row_clauses) = build_dummy_row(c_idx, &full_range, &mut pool, &mut meaning);
            // (5.9): ダミー行の合計は ちょうど d - n_c
            let k_dummy = d - n_c;
            clauses.extend(row_clauses);
            let dummy_vars: Vec<i32> = full_range.iter().map(|val| row[val]).collect();
            clauses.extend(exact_k(&dummy_vars, k_dummy, &mut pool));
            Some(row)
        } else {
            None
        };

        // (5.10): 1..=d の各値について，制約内の実変数 + ダミー行(あれば)
        // の中でちょうど1つが真。ドメイン外の p_ij は単位節で偽に固定
        // 済みなので，ここでは単純に全変数を対象にすればよい。
        for &val in &full_range {
            let mut col_vars: Vec<i32> = vars_c
                .iter()
                .map(|v_idx| *p[v_idx].get(&val).unwrap())
                .collect();
            if let Some(ref row) = dummy_row {
                col_vars.push(row[&val]);
            }
            clauses.extend(exact_one(&col_vars, &mut pool));
        }
    }

    // 補助変数として意味づけされていない変数は auxiliary として記録する。
    let total_vars = pool.total();

    let (cnf_path, map_path) = output_paths(&input_path);

    // --- CNF (DIMACS) 出力 ---
    let mut cnf = String::new();
    cnf.push_str("c Ohno et al. proposed method 4 encoding\n");
    cnf.push_str(&format!("p cnf {} {}\n", total_vars, clauses.len()));
    for clause in &clauses {
        let line: Vec<String> = clause.iter().map(|l| l.to_string()).collect();
        cnf.push_str(&line.join(" "));
        cnf.push_str(" 0\n");
    }
    fs::File::create(&cnf_path)
        .and_then(|mut f| f.write_all(cnf.as_bytes()))
        .unwrap_or_else(|e| panic!("CNFファイル {} の書き込みに失敗しました: {}", cnf_path, e));

    // --- map (変数の意味) 出力 ---
    let mut map = String::new();
    map.push_str("# sat_var meaning\n");
    for v in 1..=total_vars {
        let m = meaning
            .get(&v)
            .cloned()
            .unwrap_or_else(|| "auxiliary".to_string());
        map.push_str(&format!("{} {}\n", v, m));
    }
    fs::File::create(&map_path)
        .and_then(|mut f| f.write_all(map.as_bytes()))
        .unwrap_or_else(|e| panic!("mapファイル {} の書き込みに失敗しました: {}", map_path, e));

    // --- clasp で全解列挙し，結果を result.txt に書き込む ---
    // sat変数 -> (x_i, value) の逆引き表(ダミー行・補助変数は含まれない)
    let mut reverse: HashMap<i32, (i32, i32)> = HashMap::new();
    for (&xi, row) in &p {
        for (&val, &satvar) in row {
            reverse.insert(satvar, (xi, val));
        }
    }

    let clasp_raw = clasp::run_all_models(&cnf_path);
    let solutions = clasp::parse_solutions(&clasp_raw);

    let result_path = Path::new(&cnf_path)
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join("result.txt")
        .to_string_lossy()
        .into_owned();
    fs::File::create(&result_path)
        .and_then(|mut f| f.write_all(clasp_raw.as_bytes()))
        .unwrap_or_else(|e| panic!("結果ファイル {} の書き込みに失敗しました: {}", result_path, e));

    let mut decode = String::new();
    decode.push_str("# decoded solutions\n");
    let mut distinct: Vec<Vec<(i32, i32)>> = Vec::new();
    for (idx, literals) in solutions.iter().enumerate() {
        let assignment = dyscription::decode_solution(literals, &reverse);
        let parts: Vec<String> = assignment
            .iter()
            .map(|(xi, val)| format!("x{}={}", xi, val))
            .collect();
        decode.push_str(&format!("Answer {}: {}\n", idx + 1, parts.join(", ")));
        if !distinct.contains(&assignment) {
            distinct.push(assignment);
        }
    }

    // clasp が数える解の個数には，どの節にも現れない補助変数の自由度
    // ぶんの重複が含まれうる(実変数への割り当てとしては同一の解)。
    // 重複を除いた実際の解の一覧も末尾に付記する。
    decode.push_str(&format!(
        "\n# distinct assignments ({} / {} raw models)\n",
        distinct.len(),
        solutions.len()
    ));
    for (idx, assignment) in distinct.iter().enumerate() {
        let parts: Vec<String> = assignment
            .iter()
            .map(|(xi, val)| format!("x{}={}", xi, val))
            .collect();
        decode.push_str(&format!("Distinct {}: {}\n", idx + 1, parts.join(", ")));
    }

    let decode_path = Path::new(&cnf_path)
        .parent()
        .unwrap_or_else(|| Path::new(""))
        .join("decode.txt")
        .to_string_lossy()
        .into_owned();
    fs::File::create(&decode_path)
        .and_then(|mut f| f.write_all(decode.as_bytes()))
        .unwrap_or_else(|e| panic!("復号ファイル {} の書き込みに失敗しました: {}", decode_path, e));

    println!(
        "出力: {} (clasp解数: {})",
        result_path,
        solutions.len()
    );
    println!(
        "出力: {} (実際に異なる解: {})",
        decode_path,
        distinct.len()
    );

    println!(
        "入力: {} (制約数: {}, 変数数: {}, d={})",
        input_path,
        constraints.len(),
        used_vars.len(),
        d
    );
    println!("出力: {} (変数数: {}, 節数: {})", cnf_path, total_vars, clauses.len());
    println!("出力: {}", map_path);
}
