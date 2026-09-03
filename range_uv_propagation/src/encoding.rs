pub fn at_most_k(input: Vec<i32>, n: i32, mut m: i32, k: i32) -> (Vec<Vec<i32>>, i32) {
    let mut result: Vec<Vec<i32>> = Vec::new();

    // 入力が空の場合は何もせず返す
    if input.is_empty() {
        return (result, m);
    }

    let mut s = vec![vec![0; k.try_into().unwrap()]; input.len()];

    for i in 0..input.len() {
        for j in 0..k {
            // バグ修正 #2: 元のコードでは補助変数が衝突する可能性があったため修正。
            // (i * k) + j で一意なオフセットを計算する。
            s[i][j as usize] = n + m + (i as i32 * k) + j + 1;
        }
    }

    result.push(vec![-input[0], s[0][0]]);
    if k >= 2 {
        // バグ修正 #3: i=0 の場合に、2番目以降のカウンタ変数をfalseにする制約が漏れていた。
        for j in 1..k as usize {
            result.push(vec![-s[0][j]]);
        }
    }

    // input.len()が小さい場合に備えて saturating_sub を使い、ループがアンダーフローしないようにする
    for i in 1..input.len().saturating_sub(1) {
        result.push(vec![-input[i], s[i][0]]);
        result.push(vec![-s[i - 1][0], s[i][0]]);
        if k >= 2 {
            for j in 1..k {
                result.push(vec![
                    -input[i],
                    -s[i - 1][(j as usize) - 1],
                    s[i][j as usize],
                ]);
                result.push(vec![-s[i - 1][j as usize], s[i][j as usize]]);
            }
        }
        result.push(vec![-input[i], -s[i - 1][k as usize - 1]]);
    }

    // ★★★ パニック修正 #1: この節は入力が2つ以上ある場合のみ追加する ★★★
    if input.len() > 1 {
        let last_idx = input.len() - 1;
        result.push(vec![
            -input[last_idx],
            -s[last_idx - 1][k as usize - 1],
        ]);
    }

    // 使用した補助変数の総数を正しくmに加算する
    m = m + input.len() as i32 * k;
    return (result, m);
}

// at-least-k は，否定リテラルに対する at-most-(len-k) と等価であることを利用する。
// (¬x_1 + ... + ¬x_len <= len - k) <=> (x_1 + ... + x_len >= k)
pub fn at_least_k(input: Vec<i32>, n: i32, m: i32, k: i32) -> (Vec<Vec<i32>>, i32) {
    let len = input.len() as i32;

    if k <= 0 {
        // 常に成り立つ
        return (Vec::new(), m);
    }
    if k > len {
        // 充足不能 (空節)
        return (vec![Vec::new()], m);
    }
    if k == len {
        // 全て真でなければならない
        let result: Vec<Vec<i32>> = input.iter().map(|&lit| vec![lit]).collect();
        return (result, m);
    }

    let negated: Vec<i32> = input.iter().map(|&lit| -lit).collect();
    at_most_k(negated, n, m, len - k)
}

pub fn at_least_one(input: &Vec<i32>) -> Vec<i32> {
    let mut result: Vec<i32> = Vec::new();
    for i in 0..input.len() {
        result.push(input[i]);
    }

    return result;
}

// exact-k は at-most-k と at-least-k の連言で表す。
pub fn exact_k(input: Vec<i32>, n: i32, m: i32, k: i32) -> (Vec<Vec<i32>>, i32) {
    let len = input.len() as i32;

    if k < 0 || k > len {
        // 充足不能 (空節)
        return (vec![Vec::new()], m);
    }
    if k == 0 {
        let result: Vec<Vec<i32>> = input.iter().map(|&lit| vec![-lit]).collect();
        return (result, m);
    }
    if k == len {
        let result: Vec<Vec<i32>> = input.iter().map(|&lit| vec![lit]).collect();
        return (result, m);
    }

    let (mut clauses, m1) = at_most_k(input.clone(), n, m, k);
    let (at_least_clauses, m2) = at_least_k(input, n, m1, k);
    clauses.extend(at_least_clauses);

    (clauses, m2)
}

// exact-one は at-most-one (at_most_k with k=1) と at-least-one (単一節) の連言で表す。
// exact_k(input, n, m, 1) でも表現できるが，at-least-one は単一節で済むためこちらの方が効率的。
pub fn exact_one(input: Vec<i32>, n: i32, m: i32) -> (Vec<Vec<i32>>, i32) {
    if input.is_empty() {
        // 割り当てる値がない場合は充足不能
        return (vec![Vec::new()], m);
    }

    let (mut clauses, m1) = at_most_k(input.clone(), n, m, 1);
    clauses.push(at_least_one(&input));

    (clauses, m1)
}
