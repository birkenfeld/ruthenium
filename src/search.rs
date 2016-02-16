// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

use std::cmp::min;
use std::path::Path;

#[cfg(feature = "pcre")]
use pcre::Regex;
#[cfg(not(feature = "pcre"))]
use regex::bytes::Regex;

use options::{Casing, Opts};

/// Represents a line that matched the pattern (maybe multiple times).
#[derive(Debug)]
pub struct Match {
    /// Line number in the file
    pub lineno: usize,
    /// Line text
    pub line: Vec<u8>,
    /// Spans (start, end) of matching parts in the line
    pub spans: Vec<(usize, usize)>,
    /// Context lines before the matched line
    pub before: Vec<Vec<u8>>,
    /// Context lines after the matched line
    pub after: Vec<Vec<u8>>,
}

impl Match {
    fn new(lineno: usize, line: &[u8], spans: Vec<(usize, usize)>) -> Match {
        Match {
            lineno: lineno,
            line: line.into(),
            spans: spans,
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
    Regex::new(&pattern).unwrap()
}

/// Return normalized path: get rid of leading ./ and make leading // into /.
fn normalized_path(path: &Path) -> String {
    let s = path.to_string_lossy();
    if s.starts_with("./") {
        String::from(&s[2..])
    } else if s.starts_with("//") {
        String::from(&s[1..])
    } else {
        s.into_owned()
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

/// Iterator-like struct for collecting lines within a u8 buffer.
struct Lines<'a> {
    buf: &'a [u8],
    lines: Vec<&'a [u8]>,
    start: usize,
    lineno: usize,
}

impl<'a> Lines<'a> {
    pub fn new(buf: &[u8]) -> Lines {
        Lines { buf: buf, lines: Vec::new(), start: 0, lineno: 0 }
    }

    /// Get next line in the main iteration.
    pub fn next(&mut self) -> Option<(usize, &'a [u8])> {
        let lno = self.lineno;
        self.lineno += 1;
        if lno < self.lines.len() {
            Some((lno, self.lines[lno]))
        } else if self.advance(lno) {
            Some((lno, self.lines[lno]))
        } else {
            None
        }
    }

    /// Get an arbitrary line as a string.
    pub fn get_line(&mut self, lineno: usize) -> Option<Vec<u8>> {
        if self.advance(lineno) {
            Some(self.lines[lineno].to_vec())
        } else {
            None
        }
    }

    /// Advance the line detection until we have at least need_idx+1 lines.
    /// Return false if EOF was reached before given number of lines.
    fn advance(&mut self, need_idx: usize) -> bool {
        while self.lines.len() < need_idx + 1 {
            if self.start >= self.buf.len() {
                return false;
            }
            let end = self.buf[self.start..].iter()
                                            .position(|&x| x == b'\n')
                                            .unwrap_or(self.buf.len() - self.start);
            let line = &self.buf[self.start..self.start+end];
            self.lines.push(line);
            self.start += end + 1;
        }
        true
    }
}

/// Search a single file (represented as a u8 buffer) for matching lines.
pub fn search(regex: &Regex, opts: &Opts, path: &Path, buf: &[u8]) -> FileResult {
    let len = buf.len();
    let mut result = FileResult::new(normalized_path(path));
    result.has_context = opts.before > 0 || opts.after > 0;
    // binary file?
    if is_binary(buf, len) {
        result.is_binary = true;
        // if we care for binaries at all
        if opts.do_binaries {
            // XXX: obviously the from_utf8 will fail for binary files
            if regex.is_match(buf) {
                // found a match: create a dummy match object, and
                // leave it there (we never need more info than
                // "matched" or "didn't match")
                result.matches.push(Match::new(0, &[], Vec::new()));
            }
        }
    } else {
        let mut lines = Lines::new(buf);
        while let Some((lineno, line)) = lines.next() {
            let mut spans = Vec::new();
            if let Some(span) = regex.find(line) {
                let mut searchfrom = span.1;
                // create a match object for this line (lineno is 1-based)
                spans.push(span);
                // search for further matches in this line
                while let Some((i0, i1)) = regex.find(&line[searchfrom..]) {
                    spans.push((searchfrom + i0, searchfrom + i1));
                    searchfrom += i1;
                }
            }
            if opts.invert != spans.is_empty() {
                // no match
                continue;
            }
            let mut m = Match::new(lineno + 1, line, spans);

            // collect "before" context for this match
            if opts.before > 0 {
                for lno in lineno.saturating_sub(opts.before)..lineno {
                    m.before.push(lines.get_line(lno).unwrap());
                }
            }
            // collect "after" context for this match
            if opts.after > 0 {
                for lno in lineno+1..lineno+opts.after+1 {
                    if let Some(line) = lines.get_line(lno) {
                        m.after.push(line);
                    }
                }
            }
            result.matches.push(m);

            if opts.only_files.is_some() {
                // need only one match per file for this mode
                break;
            } else if result.matches.len() >= opts.max_count {
                break;
            }
        }
    }
    result
}
