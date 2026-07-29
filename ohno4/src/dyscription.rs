use std::collections::HashMap;

/// clasp が出力した解(リテラルの真偽割り当て)を，実変数 x_i = value の
/// 形に復号する。reverse マップに含まれない変数(ダミー行・補助変数)は
/// 無視する。
pub fn decode_solution(literals: &Vec<i32>, reverse: &HashMap<i32, (i32, i32)>) -> Vec<(i32, i32)> {
    let mut assignment: Vec<(i32, i32)> = literals
        .iter()
        .filter(|&&lit| lit > 0)
        .filter_map(|lit| reverse.get(lit).copied())
        .collect();
    assignment.sort_unstable();
    assignment
}
