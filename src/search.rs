// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

use std::cmp::min;
use std::path::Path;
use std::sync::mpsc::Sender;
use regex_dfa::Program as Regex;

use options::Opts;

#[derive(Debug)]
pub struct Match {
    pub lineno: usize,
    pub line: String,
    pub spans: Vec<(usize, usize)>,
}

impl Match {
    fn new(lineno: usize, line: &str) -> Match {
        Match {
            lineno: lineno,
            line: line.into(),
            spans: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct FileResult {
    pub fname: String,
    pub is_binary: bool,
    pub matches: Vec<Match>,
}

impl FileResult {
    fn new(fname: String) -> FileResult {
        FileResult {
            fname: fname,
            is_binary: false,
            matches: Vec::new(),
        }
    }
}

pub fn create_rx(pattern: &str, literal: bool) -> Regex {
    let mut pattern = pattern.to_owned();
    if literal {
        const ESCAPE: &'static str = ".?*+|^$(){}[]\\";
        pattern = pattern.chars().map(|c| {
            if ESCAPE.find(c).is_some() {
                format!("\\{}", c)
            } else {
                format!("{}", c)
            }
        }).collect();
    }
    Regex::from_regex(&pattern).unwrap()
}

pub fn is_binary(buf: &[u8], len: usize) -> bool {
    if len == 0 {
        return false;
    }
    if len >= 3 && &buf[0..3] == b"\xEF\xBB\xBF" {
        // UTF-8 BOM
        return false;
    }
    let n = min(512, len);
    for b in buf[..n].iter() {
        if *b == b'\x00' {
            return true;  // null byte always means binary
        }
    }
    false
}

pub fn search(chan: Sender<FileResult>, regex: &Regex, opts: &Opts,
              path: &Path, buf: &[u8]) -> usize {
    let len = buf.len();
    let mut matches = 0;
    let mut result = FileResult::new(path.to_string_lossy().into_owned());
    if is_binary(buf, len) {
        result.is_binary = true;
        if opts.do_binaries {
            if let Ok(content) = ::std::str::from_utf8(buf) {
                if let Some((_, _)) = regex.shortest_match(&content) {
                    result.matches.push(Match::new(0, ""));
                }
            }
        }
    } else {
        let mut start = 0;
        let mut lineno = 0;
        while start < len {
            lineno += 1;
            let end = buf[start..].iter().position(|&x| x == b'\n').unwrap_or(len - start);
            let line = &buf[start..start+end];
            if let Ok(line) = ::std::str::from_utf8(line) {
                if let Some(idx) = regex.shortest_match(&line) {
                    let mut searchfrom = idx.1;
                    let mut m = Match::new(lineno, line);
                    m.spans.push(idx);
                    while let Some((i0, i1)) = regex.shortest_match(&line[searchfrom..]) {
                        m.spans.push((searchfrom + i0, searchfrom + i1));
                        searchfrom += i1;
                    }
                    result.matches.push(m);
                    matches += 1;
                    if opts.only_files.is_some() {
                        // need only one match for this mode
                        break;
                    }
                }
            }
            start += end + 1;
        }
    }
    chan.send(result).unwrap();
    matches
}
