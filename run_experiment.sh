#!/bin/bash
# Range Consistency / Range-Unused-Value Propagation の Hall 区間サイズ制限実験用スクリプト。
#
# 使い方:
#   ./run_experiment.sh range_consistency        # clasp (デフォルト) で実行
#   ./run_experiment.sh range_uv_propagation
#   SOLVER=--cadical ./run_experiment.sh range_consistency
#   SOLVER="--cadical --server" ./run_experiment.sh range_consistency
#   QUEENS="8 9 10" ./run_experiment.sh range_consistency   # 対象サイズを絞る
#
# 出力: <crate>/logs/queen<N>_<条件>.log に time の結果と標準出力をまとめて書き出す。

set -eu

CRATE="${1:?crate名 (range_consistency または range_uv_propagation) を指定してください}"
SOLVER="${SOLVER:---clasp}"
QUEENS="${QUEENS:-5 6 7 8 9 10 11 12}"
TIMEOUT="${TIMEOUT:-600}"

cd "$(dirname "$0")/${CRATE}"
cargo build --release
mkdir -p logs

BIN="./target/release/${CRATE}"

run() {
  local label="$1"
  shift
  echo "=== queen_${n}: ${label} ==="
  { time timeout "${TIMEOUT}" "${BIN}" "../problem/queen_${n}.txt" --queen ${SOLVER} "$@" ; } \
    &> "logs/queen${n}_${label}.log" || echo "  -> 失敗またはタイムアウト (logs/queen${n}_${label}.log 参照)"
}

for n in ${QUEENS}; do
  run "all"                       # フィルタなし (フル分解)
  for k in 1 3 5 7 9; do
    run "under${k}" --under "$k"  # 論文の HI_k に対応
  done
  for i in 3 4 5 6 7 8 9; do
    run "only${i}" --only "$i"
  done
  run "max" --max
done

echo "完了: ${CRATE}/logs/ 以下を確認してください。"
