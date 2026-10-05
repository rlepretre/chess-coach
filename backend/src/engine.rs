//! Minimal UCI client: starts an engine process and asks it to analyse positions.
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

pub const LINES: u32 = 3;

/// One engine line, scored from the point of view of the side to move.
pub struct RawLine {
    pub rank: u32,
    pub cp: Option<i32>,
    pub mate: Option<i32>,
    pub pv: Vec<String>, // UCI moves
}

pub struct Uci {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Uci {
    pub fn start(command: &str) -> std::io::Result<Uci> {
        let mut child = Command::new(command)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;
        let stdin = child.stdin.take().unwrap();
        let stdout = BufReader::new(child.stdout.take().unwrap());
        let mut uci = Uci { child, stdin, stdout };
        uci.send("uci")?;
        uci.wait_for("uciok")?;
        uci.send(&format!("setoption name MultiPV value {LINES}"))?;
        uci.send("isready")?;
        uci.wait_for("readyok")?;
        Ok(uci)
    }

    fn send(&mut self, line: &str) -> std::io::Result<()> {
        writeln!(self.stdin, "{line}")?;
        self.stdin.flush()
    }

    fn read_line(&mut self) -> std::io::Result<String> {
        let mut line = String::new();
        if self.stdout.read_line(&mut line)? == 0 {
            return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "engine closed"));
        }
        Ok(line.trim().to_string())
    }

    fn wait_for(&mut self, expected: &str) -> std::io::Result<()> {
        while self.read_line()? != expected {}
        Ok(())
    }

    /// Returns the deepest line the engine reported for each rank, best first.
    pub fn analyse(&mut self, fen: &str, depth: u32) -> std::io::Result<Vec<RawLine>> {
        self.send(&format!("position fen {fen}"))?;
        self.send(&format!("go depth {depth}"))?;
        let mut best: BTreeMap<u32, (u32, RawLine)> = BTreeMap::new();
        loop {
            let line = self.read_line()?;
            if line.starts_with("bestmove") {
                return Ok(best.into_values().map(|(_, l)| l).collect());
            }
            if let Some((d, l)) = parse_info(&line) {
                if best.get(&l.rank).map_or(true, |(old, _)| d >= *old) {
                    best.insert(l.rank, (d, l));
                }
            }
        }
    }
}

impl Drop for Uci {
    fn drop(&mut self) {
        let _ = self.send("quit");
        let _ = self.child.wait();
    }
}

/// Parses `info depth 12 multipv 2 score cp 31 pv e2e4 e7e5 ...`. Ignores bound estimates.
fn parse_info(line: &str) -> Option<(u32, RawLine)> {
    let t: Vec<&str> = line.split_whitespace().collect();
    if t.first() != Some(&"info") || t.contains(&"lowerbound") || t.contains(&"upperbound") {
        return None;
    }
    let after = |key: &str| t.iter().position(|w| *w == key).map(|i| i + 1);
    let depth = t.get(after("depth")?)?.parse().ok()?;
    let rank = after("multipv").and_then(|i| t.get(i)?.parse().ok()).unwrap_or(1);
    let s = after("score")?;
    let value: i32 = t.get(s + 1)?.parse().ok()?;
    let (cp, mate) = match *t.get(s)? {
        "cp" => (Some(value), None),
        "mate" => (None, Some(value)),
        _ => return None,
    };
    let pv = t[after("pv")?..].iter().map(|m| m.to_string()).collect();
    Some((depth, RawLine { rank, cp, mate, pv }))
}
