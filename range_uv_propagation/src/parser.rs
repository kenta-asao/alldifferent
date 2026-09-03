use std::collections::HashMap;
use std::fs;
use std::io;

// 入力ファイルの形式 (example.txt):
//   # comment
//   domain <d>                 -- 全体の値域 1..=d を宣言 (1行のみ想定)
//   domain <var> <v1> <v2> ... 0  -- 変数 var の値域を {v1, v2, ...} に制限
//   alldifferent <v1> <v2> ... 0   -- alldifferent(v1, v2, ...) 制約
pub struct Problem {
    pub d: i32,
    pub domains: HashMap<i32, Vec<i32>>,
    pub alldiffs: Vec<Vec<i32>>,
}

pub fn parse_file(path: &str) -> io::Result<Problem> {
    let content = fs::read_to_string(path)?;

    let mut d: i32 = 0;
    let mut domains: HashMap<i32, Vec<i32>> = HashMap::new();
    let mut alldiffs: Vec<Vec<i32>> = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let tokens: Vec<&str> = line.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }

        match tokens[0] {
            "domain" => {
                let rest = &tokens[1..];
                if rest.len() == 1 {
                    // domain <d>
                    d = rest[0].parse().map_err(|_| {
                        io::Error::new(io::ErrorKind::InvalidData, format!("invalid domain line: {}", line))
                    })?;
                } else {
                    // domain <var> <v1> ... 0
                    if rest.is_empty() || rest[rest.len() - 1] != "0" {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("domain line must end with 0: {}", line),
                        ));
                    }
                    let var: i32 = rest[0].parse().map_err(|_| {
                        io::Error::new(io::ErrorKind::InvalidData, format!("invalid domain line: {}", line))
                    })?;
                    let values: Vec<i32> = rest[1..rest.len() - 1]
                        .iter()
                        .map(|s| s.parse::<i32>())
                        .collect::<Result<Vec<i32>, _>>()
                        .map_err(|_| {
                            io::Error::new(io::ErrorKind::InvalidData, format!("invalid domain line: {}", line))
                        })?;
                    domains.insert(var, values);
                }
            }
            "alldifferent" => {
                let rest = &tokens[1..];
                if rest.is_empty() || rest[rest.len() - 1] != "0" {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidData,
                        format!("alldifferent line must end with 0: {}", line),
                    ));
                }
                let vars: Vec<i32> = rest[..rest.len() - 1]
                    .iter()
                    .map(|s| s.parse::<i32>())
                    .collect::<Result<Vec<i32>, _>>()
                    .map_err(|_| {
                        io::Error::new(io::ErrorKind::InvalidData, format!("invalid alldifferent line: {}", line))
                    })?;
                alldiffs.push(vars);
            }
            other => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unknown keyword '{}' in line: {}", other, line),
                ));
            }
        }
    }

    if d <= 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "domain size d was not declared (missing 'domain <d>' line)",
        ));
    }

    Ok(Problem { d, domains, alldiffs })
}
