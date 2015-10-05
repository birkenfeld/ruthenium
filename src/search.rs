// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

use std::cmp::min;
use std::path::Path;
use std::str;
use regex_dfa::Program as Regex;

use options::{Casing, Opts};

/// Represents a line that matched the pattern (maybe multiple times).
#[derive(Debug)]
pub struct Match {
    /// Line number in the file
    pub lineno: usize,
    /// Line text
    pub line: String,
    /// Spans (start, end) of matching parts in the line
    pub spans: Vec<(usize, usize)>,
    /// Context lines before the matched line
    pub before: Vec<String>,
    /// Context lines after the matched line
    pub after: Vec<String>,
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

/// Represents all matches from a single file.
#[derive(Debug)]
pub struct FileResult {
    /// File name, relative to initial argument
    pub fname: String,
    /// Is the file binary?  If yes, matches contains 0 or 1 element
    pub is_binary: bool,
    /// Do we provide (and print) context lines?
    pub has_context: bool,
    /// Matches relevant for printing
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

/// Create a regular expression to search for matches from the given options.
///
/// The final regex is determined by several options, such as casing options
/// and options to take the search string literally.
pub fn create_rx(opts: &Opts) -> Regex {
    let mut pattern = opts.pattern.to_owned();
    if opts.literal {
        // escape regex meta-chars and create a normal pattern
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
        // smart casing: only case-insensitive when pattern contains no uppercase
        if !pattern.chars().any(|c| c.is_uppercase()) {
            pattern = format!("(?i){}", pattern);
        }
    }
    Regex::from_regex(&pattern).unwrap()
}

/// Return normalized path: get rid of leading ./ and make leading // into /.
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

/// Check file for binary-ness.
///
/// Currently only null-bytes are recognized to constitute binary file content.
/// However, this clashes with UTF-16 and UTF-32, so a more clever heuristic is
/// required at some point.
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

/// Search a single file (represented as a u8 buffer) for matching lines.
pub fn search(regex: &Regex, opts: &Opts, path: &Path, buf: &[u8]) -> FileResult {
    let len = buf.len();
    let mut matches = 0;
    let mut result = FileResult::new(normalized_path(path));
    result.has_context = opts.before > 0 || opts.after > 0;
    // binary file?
    if is_binary(buf, len) {
        result.is_binary = true;
        // if we care for binaries at all
        if opts.do_binaries {
            // XXX: obviously the from_utf8 will fail for binary files
            if let Ok(content) = str::from_utf8(buf) {
                if let Some((_, _)) = regex.shortest_match(&content) {
                    // found a match: create a dummy match object, and
                    // leave it there (we never need more info than
                    // "matched" or "didn't match")
                    result.matches.push(Match::new(0, ""));
                }
            }
        }
    } else {
        // cache read lines for context
        let mut lines = Vec::new();
        let mut start = 0;
        let mut lineno = 0;
        let mut needs_ctxt: Vec<(Match, usize)> = Vec::new();
        while start < len {
            lineno += 1;
            // find the end of this line (or EOF)
            let end = buf[start..].iter().position(|&x| x == b'\n')
                                         .unwrap_or(len - start);
            let line = &buf[start..start+end];
            lines.push(line);

            // collect "after" context for previously seen matches
            // XXX: refactor this, it uses too many vecs, and copies lines over
            // and over again in pathological cases
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

            // XXX: we should not have to do from_utf8 but all current regex engines
            // work on Unicode strings, so they need a str
            if let Ok(line) = str::from_utf8(line) {
                if let Some(idx) = regex.shortest_match(&line) {
                    let mut searchfrom = idx.1;
                    // create a match object for this line
                    let mut m = Match::new(lineno, line);
                    m.spans.push(idx);
                    // search for further matches in this line
                    while let Some((i0, i1)) = regex.shortest_match(&line[searchfrom..]) {
                        m.spans.push((searchfrom + i0, searchfrom + i1));
                        searchfrom += i1;
                    }
                    matches += 1;
                    if opts.only_files.is_some() {
                        // need only one match per file for this mode
                        result.matches.push(m);
                        break;
                    } else if matches >= opts.max_count {
                        // XXX: must still collect "after" context!
                        break;
                    }

                    // collect "before" context for this match
                    if opts.before > 0 {
                        for i in 0..opts.before {
                            let j = opts.before - i;
                            if j < lineno {
                                m.before.push(String::from_utf8_lossy(lines[lineno-j-1]).into_owned());
                            }
                        }
                    }

                    // mark this match as maybe needing "after" context
                    if opts.after > 0 {
                        needs_ctxt.push((m, opts.after));
                    } else {
                        result.matches.push(m);
                    }
                }
            }
            start += end + 1;
        }

        // no more "after" context lines for these outstanding matches
        result.matches.extend(needs_ctxt.into_iter().map(|(m, _)| m));
    }
    result
}
