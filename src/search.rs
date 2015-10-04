// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

use std::cmp::min;
use std::path::Path;
use std::str;
use regex_dfa::Program as Regex;

use options::{Casing, Opts};

#[derive(Debug)]
pub struct Match {
    pub lineno: usize,
    pub line: String,
    pub before: Vec<String>,
    pub after: Vec<String>,
    pub spans: Vec<(usize, usize)>,
}

impl Match {
    fn new(lineno: usize, line: &str) -> Match {
        Match {
            lineno: lineno,
            line: line.into(),
            spans: Vec::new(),
            before: Vec::new(),
            after: Vec::new(),
        }
    }
}

#[derive(Debug)]
pub struct FileResult {
    pub fname: String,
    pub is_binary: bool,
    pub has_context: bool,
    pub matches: Vec<Match>,
}

impl FileResult {
    fn new(fname: String) -> FileResult {
        FileResult {
            fname: fname,
            is_binary: false,
            has_context: false,
            matches: Vec::new(),
        }
    }
}

pub fn create_rx(opts: &Opts) -> Regex {
    let mut pattern = opts.pattern.to_owned();
    if opts.literal {
        const ESCAPE: &'static str = ".?*+|^$(){}[]\\";
        pattern = pattern.chars().map(|c| {
            if ESCAPE.find(c).is_some() {
                format!("\\{}", c)
            } else {
                format!("{}", c)
            }
        }).collect();
    }
    if let Casing::Insensitive = opts.casing {
        pattern = format!("(?i){}", pattern);
    } else if let Casing::Smart = opts.casing {
        if !pattern.chars().any(|c| c.is_uppercase()) {
            pattern = format!("(?i){}", pattern);
        }
    }
    Regex::from_regex(&pattern).unwrap()
}

fn normalized_path(path: &Path) -> String {
    let s = path.to_string_lossy();
    if s.starts_with("./") {
        String::from(&s[2..])
    } else if s.starts_with("//") {
        String::from(&s[1..])
    } else {
        String::from(&s[..])
    }
}

fn is_binary(buf: &[u8], len: usize) -> bool {
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

pub fn search(regex: &Regex, opts: &Opts, path: &Path, buf: &[u8]) -> FileResult {
    let len = buf.len();
    let mut matches = 0;
    let mut result = FileResult::new(normalized_path(path));
    result.has_context = opts.before > 0 || opts.after > 0;
    if is_binary(buf, len) {
        result.is_binary = true;
        if opts.do_binaries {
            if let Ok(content) = str::from_utf8(buf) {
                if let Some((_, _)) = regex.shortest_match(&content) {
                    result.matches.push(Match::new(0, ""));
                }
            }
        }
    } else {
        let mut lines = Vec::new();
        let mut start = 0;
        let mut lineno = 0;
        let mut needs_ctxt: Vec<(Match, usize)> = Vec::new();
        while start < len {
            lineno += 1;
            let end = buf[start..].iter().position(|&x| x == b'\n')
                                         .unwrap_or(len - start);
            let line = &buf[start..start+end];
            lines.push(line);

            // XXX refactor this, it uses too many vecs.
            let mut remove = Vec::new();
            for (i, t) in needs_ctxt.iter_mut().enumerate() {
                t.0.after.push(String::from_utf8_lossy(line).into_owned());
                t.1 -= 1;
                if t.1 == 0 {
                    remove.push(i);
                }
            }
            for i in &remove {
                result.matches.push(needs_ctxt.remove(*i).0);
            }

            if let Ok(line) = str::from_utf8(line) {
                if let Some(idx) = regex.shortest_match(&line) {
                    let mut searchfrom = idx.1;
                    let mut m = Match::new(lineno, line);
                    m.spans.push(idx);
                    while let Some((i0, i1)) = regex.shortest_match(&line[searchfrom..]) {
                        m.spans.push((searchfrom + i0, searchfrom + i1));
                        searchfrom += i1;
                    }
                    matches += 1;
                    if opts.only_files.is_some() {
                        // need only one match for this mode
                        break;
                    } else if matches >= opts.max_count {
                        break; // XXX must collect context!
                    }

                    if opts.before > 0 {
                        for i in 0..opts.before {
                            let j = opts.before - i;
                            if j < lineno {
                                m.before.push(String::from_utf8_lossy(lines[lineno-j-1]).into_owned());
                            }
                        }
                    }

                    if opts.after > 0 {
                        needs_ctxt.push((m, opts.after));
                    } else {
                        result.matches.push(m);
                    }
                }
            }
            start += end + 1;
        }

        result.matches.extend(needs_ctxt.into_iter().map(|(m, _)| m));
    }
    result
}
