# Alldifferent

alldifferent 制約の効果的な Decomposition に関する研究用リポジトリ。

alldifferent 制約を CNF (SAT) に符号化する3つの分解手法を，それぞれ独立した Rust クレートとして実装している。
いずれも「domain/alldifferent 形式のテキストファイルを読み込み → CNF に符号化 → clasp か cadical で求解 →
（`--decode` 指定時のみ）元の変数の割り当てに復号」という同じ流れで動作し，コマンドライン引数の形式も共通。

| クレート | 手法 |
| --- | --- |
| `uv_propagation` | Unused Value Propagation（大野らの提案手法4） |
| `range_consistency` | Range Consistency（Bessiere らの分解，IJCAI 2009 Theorem 1） |
| `range_uv_propagation` | 上記2つを組み合わせた Range-Unused-Value Propagation（本研究の提案） |
| `queen` | ベンチマーク用のクイーングラフ彩色問題インスタンス生成器 |

各クレートは Cargo ワークスペースにはなっておらず，それぞれ独立にビルド・実行する。

## ビルド

```
cd uv_propagation          # または range_consistency / range_uv_propagation
cargo build --release
```

## 共通の実行方法

```
cargo run --release -- <folder>/<problem_file> [--decode | --queen] [--clasp | --cadical] [--all] [--server] [--only N | --under N | --max]
```

- `<folder>/<problem_file>`: 入力ファイル（`example.txt` 参照，形式は後述）
- 引数がファイルパスのみ（フラグなし）の場合は，CNF への符号化・書き出しのみを行って終了する
- `--clasp`: clasp で求解する（デフォルト。`--clasp`/`--cadical` を両方指定するとエラー）
- `--cadical`: cadical で求解する（単一解探索のみ対応。`--all` と併用すると警告を出して無視される）
- `--server`: `--cadical` と併用時，ローカルビルドではなくサーバー上の cadical バイナリを使う
- `--all`: clasp を `-n 0`（全解列挙）で実行する。指定しない場合は `-n 1`（単解探索）。cadical では常に単一解のみ
- `--decode`: 求解結果を元の変数の割り当て `x_i = j` に復号し，標準出力と `decode/` フォルダに書き出す
- `--queen`: クイーングラフ彩色問題専用の復号形式（盤面表示）を使う。指定すると `--decode` も自動的に有効になる
- `--only N` / `--under N` / `--max`: `range_consistency`・`range_uv_propagation` のみで有効。range consistency
  分解を課す Hall 区間のサイズ `k = u - l + 1` を制限する（Bessiere ら (IJCAI 2009) の実験における `HI_k` に対応，
  詳細は「Range Consistency」節を参照）。3つは互いに排他で，同時に指定するとエラーになる。指定しない場合は
  従来通り全てのサイズの区間に分解を課す

実行すると，カレントディレクトリ直下に `cnf/`・`result/`・`decode/` フォルダが作成され，各手法ごとのプレフィックス
（`uvp_`／`rc_`／`rcuvp_`）を付けたファイル名で出力される（入力パスがどのフォルダにあっても，出力ファイル名は
フォルダ名を含まない `<問題名>` だけになる）。例えば `uv_propagation` で `problem/example.txt` を解くと:

```
cnf/uvp_example.cnf
result/uvp_result_example.txt
decode/uvp_decode_example.txt   # --decode 指定時のみ生成
```

実行例:

```
# CNF への符号化のみ
cargo run --release -- problem/example.txt

# clasp で単解探索 + 復号
cargo run --release -- problem/example.txt --decode

# clasp で全解列挙 + 復号
cargo run --release -- problem/example.txt --all --decode

# cadical（ローカルビルド）で求解
cargo run --release -- problem/example.txt --cadical --decode

# cadical（サーバー上のバイナリ）で求解
cargo run --release -- problem/example.txt --cadical --server --decode

# クイーングラフ彩色問題として求解し，盤面を復号表示
cargo run --release -- problem/queen_8.txt --queen --cadical
```

`--all --decode` を同時に指定した場合（clasp のみ），`result_*.txt` 中の `c Answer: N` 区切りごとに解を分けて
`Answer 1:`, `Answer 2:`, ... の形で**全ての解**を復号する。ただし基数制約エンコーディング（sequential counter
による at-most-k など）は補助変数の割り当てに自由度が残るため，本質的に同じ `x_i = j` の解が補助変数違いで
何度も列挙される点に注意（論文の実験では対称解除去などの後処理を別途行っている）。

前提として，`--clasp`（デフォルト）を使う場合は `clasp` コマンドが PATH 上にインストールされている必要がある。
`--cadical` を使う場合は `uv_propagation/src/cadical.rs` などにハードコードされたパス
（ローカル: `/home/kenta/git/programing/cadical/build/cadical`，サーバー: `/home/share/app/cadical`）に
バイナリが存在している必要がある。

## 入力ファイルの形式

`#` から始まる行はコメント。3クレート共通のパーサ（`parser.rs`）を使う。

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

同じ変数が複数の `alldifferent` に登場してもよい（数独やクイーングラフ彩色問題のように，行・列・対角線などで
同じ変数を共有する制約を表現できる）。

## Unused Value Propagation（`uv_propagation`）

大野らの alldifferent 制約のブール基数制約への符号化手法のうち，提案手法4（n+1 行目を追加する手法）を用いて
alldifferent 制約付きの問題を CNF に符号化する。

提案手法4の定義:

- `pi1 + ... + pid = 1` （各変数 x_i はちょうど1つの値をとる）
- `p(n+1)1 + ... + p(n+1)d = d - n` （追加した n+1 行目がちょうど d-n 個の値をとる）
- `p1j + ... + p(n+1)j = 1` （各値 j はちょうど1つの変数に割り当てられる）

### 参考

- 大野周亮, 「alldifferent制約のブール基数制約への符号化とその性能評価」, 神戸大学卒業論文, 2019.

## Range Consistency（`range_consistency`）

Bessiere, Katsirelos, Narodytska, Quimper, Walsh, "Decompositions of All Different, Global
Cardinality and Related Constraints" (IJCAI 2009) の Theorem 1 で示されている，AllDifferent 制約を
range consistency (RC) まで枝刈りする分解。

alldifferent 制約 `{X_i}` (要素数 `n`) に対し，`1 <= l <= u <= d` かつ `u - l < n` を満たす各区間 `[l, u]`
ごとに，補助変数 `A_{i,l,u}` (`X_i` が区間 `[l, u]` に入るかどうか) を導入し，以下の2種類の制約を課す。

- (1) `A_{i,l,u} <=> X_i ∈ [l, u]`（`p_ij` の exact-one 制約より，`OR_{j=l}^{u} p_ij` と同値）
- (2) `sum_i A_{i,l,u} <= u - l + 1`（区間 `[l, u]` はその大きさを超える数の変数を割り当てられない = Hall 区間の枝刈り）

論文では (1)(2) に関して domain consistency を課すことで，元の AllDifferent 制約が range consistent になる
ことが証明されている（Leconte の RC アルゴリズムと同等の枝刈り）。`u - l + 1 == n` の場合，(2) は変数の総数が
ちょうど `n` であることから自明に成立するため，実装では省略している。

### Hall 区間のサイズ制限（`--only`/`--under`/`--max`）

論文の実験では，全ての区間 `[l, u]` に分解を課すのではなく，区間サイズ `k = u - l + 1` を制限した簡略版
（論文中の `HI_k`: `u - l + 1 <= k` を満たす制約 (2) のみを課す）を用いて，小さい Hall 区間の検出に絞った場合の
効果を調べている。本実装ではこれを一般化し，`range_consistency`・`range_uv_propagation` の両クレートで
以下のオプションによりサイズ `k` の対象区間を制限できる（対象外の区間は補助変数 `A_{i,l,u}` ごと生成しない）。

- `--only N`: サイズがちょうど `N` の区間だけに分解を課す（論文にはない条件で，特定サイズ単独の効果を見る）
- `--under N`: サイズが `N` 以下の区間だけに分解を課す（論文の `HI_N` に相当。実験条件の一つとして
  `N = 1, 3, 5, 7, 9` を想定）
- `--max`: 各 alldifferent 制約ごとに，意味のある最大サイズ（要素数 `n` に対し `k = n - 1`。`k = n` は (2) が
  自明に成り立つため元々対象外）の区間だけに分解を課す
- 指定なし: 制限なし（全サイズの区間に分解を課す，従来通りの挙動）

`--only`/`--under`/`--max` は互いに排他で，同時に2つ以上指定するとエラーになる。`range_uv_propagation` では
これらのオプションは range consistency 分解にのみ適用され，unused value propagation 分解には影響しない。

**サイズ1の区間は常に含まれる。** サイズ1の Hall 区間（各値 `j` について「変数 `X_i` が値 `j` をとるかどうか」の
補助変数の総和が高々1，すなわち「各値はちょうど1つの変数に割り当てられる」）は，`range_consistency` クレート
単体では alldifferent 制約そのものを成立させる基本制約であり，これを除外すると分解全体が alldifferent の
必要条件を満たさなくなる（本来 UNSAT な問題が誤って SAT と判定される）。そのため `--only`/`--max` でサイズ1
以外を指定しても，サイズ1の区間はフィルタに関係なく常に追加される。`--only N`/`--max` は「基本制約 + 追加で
サイズ `N` の区間も分解する」という意味になる。

### 参考

- C. Bessiere, G. Katsirelos, N. Narodytska, C.-G. Quimper, T. Walsh,
  "Decompositions of All Different, Global Cardinality and Related Constraints", IJCAI 2009.

## Range-Unused-Value Propagation（`range_uv_propagation`）

Unused Value Propagation と Range Consistency の2つの分解を，同じ `p_ij` 変数を土台にして冗長に重ねがけした
分解（本研究の貢献）。各 alldifferent 制約に対し，range consistency 分解の (1)(2) と Unused Value
Propagation の分解（提案手法4）の両方を同時に課す。異なる伝播原理に基づく制約を組み合わせることで，
どちらか一方の分解では検出できない枝刈りをもう一方が補うことを狙っている。

## クイーングラフ彩色問題インスタンス生成器（`queen`）

大野周亮, 番原睦則, 宋剛秀, 田村直之. 「alldifferent 制約のブール基数制約への符号化手法の提案とクイーングラフ
彩色問題への応用」5.1節のモデルに基づき，クイーングラフ彩色問題のベンチマークインスタンス（domain/alldifferent
形式）を生成する。

```
cd queen
cargo run --release -- 8 9 10   # N=8,9,10 のインスタンスを生成 (引数省略時は N=5..=12)
```

生成される制約:

- 各マス `(i, j)` に対応する変数 `x_ij`（1 行に N マスある N×N 盤面）
- 対称解除去のため，1 行目 j 列目のクイーンの色を j に固定（`domain x_1j = j`）
- 各行・各列・各右下がり対角線・各右上がり対角線ごとに alldifferent 制約

出力先はリポジトリ内ではなく `/home/kenta/git/cpaior-asao/problem`（`queen_<N>.txt`）にハードコードされている点に
注意（本リポジトリの `problem/` 以下のクイーン問題ファイルは，そこから生成したものをコピーしたもの）。

`--queen` オプション付きで各手法の求解プログラムを実行すると，`x_ij` の割り当てを盤面の形に復号して表示できる。

## ソース構成

`uv_propagation` / `range_consistency` / `range_uv_propagation` の3クレートは，ほぼ同じソース構成を持つ
（`encoding.rs` の中身は共通，符号化ロジック本体は `main.rs` にクレートごとに異なる形で実装されている）。

| ファイル | 役割 |
| --- | --- |
| `src/main.rs` | 全体のオーケストレーション（パース→符号化→CNF出力→clasp/cadical実行→復号） |
| `src/parser.rs` | 入力ファイルのパーサ |
| `src/encoding.rs` | ブール基数制約 (at-most-k, at-least-k, exact-k, exact-one など) の CNF エンコーダ |
| `src/cnf.rs` | DIMACS CNF 形式でのファイル出力 |
| `src/clasp.rs` | clasp の実行 |
| `src/cadical.rs` | cadical の実行（ローカル/サーバーのバイナリを切り替え） |
| `src/decode.rs` | 求解結果を `x_i = j` の形（または `--queen` 指定時は盤面）に復号 |

`queen/src/main.rs` はクイーングラフ彩色問題のインスタンス生成のみを行う単体のバイナリ。
