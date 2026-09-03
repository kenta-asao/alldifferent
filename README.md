# Alldifferent

alldifferent 制約の効果的な Decomposition に関する研究用リポジトリ。

## Unused Value Propagation (大野らの提案手法4)

大野らの alldifferent 制約のブール基数制約への符号化手法のうち，提案手法4
（n+1 行目を追加する手法）を用いて，alldifferent 制約付きの問題を SAT (CNF) に
符号化し，clasp で求解するツール。

### ビルド

```
cd uv_propagation
cargo build --release
```

### 実行の流れ

1. 入力ファイル（`example.txt` 参照）を読み込みパースする
2. 提案手法4で alldifferent 制約を CNF に符号化し，`<入力ファイル名(拡張子なし)>.cnf` に出力する
3. `clasp -n 1 <cnf>`（`--all` 指定時は `clasp -n 0 <cnf>`）を実行し，結果を `result_<入力ファイル名(拡張子なし)>.txt` に出力する
4. `--decode` オプション付きで実行した場合，`result_*.txt` の内容を元の変数 `x_i = j` の割り当てに復号して標準出力に表示し，
   同じ内容を `decode_<入力ファイル名(拡張子なし)>.txt` にも書き出す（`--decode` を指定しない場合はこのファイルは生成されない）

```
# CNF への符号化 + clasp での求解のみ (単解探索: -n 1)
cargo run --release -- example.txt

# 求解結果を x_i = j の形に復号して表示する
cargo run --release -- example.txt --decode

# 全解列挙 (-n 0) で求解する
cargo run --release -- example.txt --all

# 全解列挙 + 復号
cargo run --release -- example.txt --all --decode
```

- `--decode`: `result_*.txt` を元の変数の割り当て `x_i = j` に復号して標準出力に表示し，`decode_*.txt` にも書き出す
- `--all`: clasp を `-n 0` で実行し，単解ではなく全解を列挙する（UNSAT かどうかの判定自体は `--all` なしでも正しく行われる。論文の実験でいう「全解探索」に対応するオプション）

`--all --decode` を同時に指定した場合，`result_*.txt` 中の `c Answer: N` 区切りごとに解を分けて `Answer 1:`, `Answer 2:`, ... の形で
**全ての解**を復号し，`decode_*.txt` にもその全件を書き出す。ただし本実装の基数制約エンコーディング（sequential counter による
at-most-k など）は補助変数の割り当てに自由度が残るため，本質的に同じ `x_i = j` の解が補助変数違いで何度も列挙される点に注意
（論文の実験では対称解除去などの後処理を別途行っている）。

出力例（`--decode` 時）:

```
CNF を書き出しました: example.cnf
clasp の実行結果を書き出しました: result_example.txt
SATISFIABLE
x1 = 1
x2 = 3
x3 = 2
```

### 入力ファイルの形式

`#` から始まる行はコメント。

```
# alldifferent
domain 5              # 値域を 1..=d (ここでは d=5) に設定
domain 1 1 3 0         # 変数1の値域を {1, 3} に制限 (末尾は 0 終端)
domain 2 1 3 0         # 変数2の値域を {1, 3} に制限
domain 3 1 2 3 0       # 変数3の値域を {1, 2, 3} に制限
alldifferent 1 2 3 0    # alldifferent(x1, x2, x3) 制約 (末尾は 0 終端)
# --- Constraints End ---
```

- `domain <d>`: 全体の値域の上限 d を宣言する（1行のみ，`d` に対し変数は 1..=d の値を取り得る）
- `domain <var> <v1> <v2> ... 0`: 変数 `var` が取り得る値を `{v1, v2, ...}` に制限する（省略した変数は 1..=d 全体が値域になる）
- `alldifferent <v1> <v2> ... 0`: `alldifferent(x_v1, x_v2, ...)` 制約を追加する（複数行書ける）

同じ変数が複数の `alldifferent` に登場してもよい（数独やクイーングラフ彩色問題のように，行・列・ブロックなどで
同じ変数を共有する制約を表現できる）。

### ソース構成

| ファイル | 役割 |
| --- | --- |
| `src/main.rs` | 全体のオーケストレーション（パース→符号化→CNF出力→clasp実行→復号） |
| `src/parser.rs` | 入力ファイルのパーサ |
| `src/encoding.rs` | ブール基数制約 (at-most-k, at-least-k, exact-k, exact-one など) の CNF エンコーダ |
| `src/cnf.rs` | DIMACS CNF 形式でのファイル出力 |
| `src/clasp.rs` | clasp の実行 |
| `src/decode.rs` | clasp の求解結果を `x_i = j` の形に復号 |

前提として `clasp` コマンドが PATH 上にインストールされている必要がある。

### 参考

- 大野周亮, 「alldifferent制約のブール基数制約への符号化とその性能評価」, 神戸大学卒業論文, 2019.
  提案手法4（n+1行目を追加する手法）の定義:
  - `pi1 + ... + pid = 1` (各変数 x_i はちょうど1つの値をとる)
  - `p(n+1)1 + ... + p(n+1)d = d - n` (追加した n+1 行目がちょうど d-n 個の値をとる)
  - `p1j + ... + p(n+1)j = 1` (各値 j はちょうど1つの変数に割り当てられる)

## Range Consistency (Bessiere らの分解)

Bessiere, Katsirelos, Narodytska, Quimper, Walsh, "Decompositions of All Different, Global
Cardinality and Related Constraints" (IJCAI 2009) の Theorem 1 で示されている，
AllDifferent 制約を range consistency (RC) まで枝刈りする分解を SAT (CNF) に符号化し，
clasp で求解するツール。使い方・入力ファイル形式・`--decode`/`--all` オプション・
ソース構成（`parser.rs`/`cnf.rs`/`clasp.rs`/`decode.rs`）は上記の Unused Value
Propagation と共通（`parser.rs`/`cnf.rs`/`clasp.rs`/`decode.rs`/`encoding.rs` はそのまま流用）。

```
cd range_consistency
cargo build --release
cargo run --release -- example.txt --decode
```

alldifferent 制約 `{X_i}` (要素数 `n`) に対し，`1 <= l <= u <= d` かつ `u - l < n` を満たす
各区間 `[l, u]` ごとに，補助変数 `A_{i,l,u}` (`X_i` が区間 `[l, u]` に入るかどうか) を導入し，
以下の2種類の制約を課す。

- (1) `A_{i,l,u} <=> X_i ∈ [l, u]`（`p_ij` の exact-one 制約より，`OR_{j=l}^{u} p_ij` と同値）
- (2) `sum_i A_{i,l,u} <= u - l + 1`（区間 `[l, u]` はその大きさを超える数の変数を割り当てられない = Hall 区間の枝刈り）

論文では (1)(2) に関して domain consistency を課すことで，元の AllDifferent 制約が
range consistent になることが証明されている（Leconte の RC アルゴリズムと同等の枝刈り）。
`u - l + 1 == n` の場合，(2) は変数の総数がちょうど `n` であることから自明に成立するため，
実装では省略している。

### 参考

- C. Bessiere, G. Katsirelos, N. Narodytska, C.-G. Quimper, T. Walsh,
  "Decompositions of All Different, Global Cardinality and Related Constraints", IJCAI 2009.
