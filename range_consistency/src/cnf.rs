use std::fs::File;
use std::io::{self, Write, BufWriter};

pub struct Cnf {
    pub num_vars: i32,
    pub clauses: Vec<Vec<i32>>,
}

impl Cnf {
    pub fn new() -> Self {
        Cnf {
            num_vars: 0,
            clauses: Vec::new(),
        }
    }

    pub fn add_clause(&mut self, clause: Vec<i32>) {
        self.clauses.push(clause);
    }

    pub fn add_clauses(&mut self, clauses: Vec<Vec<i32>>) {
        self.clauses.extend(clauses);
    }

    pub fn write_to_file(&self, path: &str) -> io::Result<()> {
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        writeln!(writer, "p cnf {} {}", self.num_vars, self.clauses.len())?;
        for clause in &self.clauses {
            let mut line = String::new();
            for lit in clause {
                line.push_str(&lit.to_string());
                line.push(' ');
            }
            line.push('0');
            writeln!(writer, "{}", line)?;
        }

        Ok(())
    }
}
