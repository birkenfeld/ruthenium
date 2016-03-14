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
    fn new(lineno: usize, line: Vec<u8>, spans: Vec<(usize, usize)>) -> Match {
        Match {
            lineno: lineno,
            line: line,
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

/// Cache for collecting line offsets and slices within a u8 buffer.
struct Lines<'a> {
    buf: &'a [u8],
    offset: usize,
    lines: Vec<(usize, &'a [u8])>,
}

impl<'a> Lines<'a> {
    pub fn new(buf: &[u8]) -> Lines {
        Lines { buf: buf, offset: 0, lines: Vec::with_capacity(100) }
    }

    /// Advance the line detection until we have at least lineno lines.
    /// Return false if EOF was reached before given number of lines.
    fn advance_to_line(&mut self, lineno: usize) -> bool {
        while self.lines.len() < lineno + 1 {
            if self.buf.len() == self.offset {
                return false;
            }
            let line = match self.buf[self.offset..].iter().position(|&x| x == b'\n') {
                Some(idx) => &self.buf[self.offset..self.offset+idx+1],
                None      => &self.buf[self.offset..self.buf.len()],
            };
            self.lines.push((self.offset, line));
            self.offset += line.len();
        }
        true
    }

    /// Advance to a given byte offset in the buffer.
    fn advance_to_offset(&mut self, offset: usize) {
        while self.offset < offset {
            let next_line = self.lines.len();
            self.advance_to_line(next_line);
        }
    }

    /// Get line number of offset.
    pub fn get_lineno(&mut self, offset: usize) -> usize {
        self.advance_to_offset(offset);
        for (n, &(o, _)) in self.lines.iter().enumerate().rev() {
            if o <= offset {
                return n;
            }
        }
        return 0;
    }

    /// Get offset of line number.
    pub fn get_offset(&mut self, lineno: usize) -> usize {
        if self.advance_to_line(lineno) {
            self.lines[lineno].0
        } else {
            self.buf.len()
        }
    }

    /// Get an arbitrary line (maybe beyond end of file) as a string.
    pub fn get_line(&mut self, lineno: usize) -> Option<Vec<u8>> {
        if self.advance_to_line(lineno) {
            Some(self.lines[lineno].1.to_vec())
        } else {
            None
        }
    }
}

/// Create a line-match for a given line with context lines determined by options.
fn create_match(lines: &mut Lines, opts: &Opts, lineno: usize) -> Match {
    let line = lines.get_line(lineno).expect("matched line missing");
    let mut new_match = Match::new(lineno + 1, line, vec![]);
    if opts.before > 0 {
        for lno in lineno.saturating_sub(opts.before)..lineno {
            new_match.before.push(lines.get_line(lno).unwrap());
        }
    }
    if opts.after > 0 {
        for lno in lineno+1..lineno+opts.after+1 {
            if let Some(line) = lines.get_line(lno) {
                new_match.after.push(line);
            }
        }
    }
    new_match
}

/// Add a new match and maybe finish
macro_rules! new_match {
    ($result:expr, $lines:expr, $opts:expr, $lineno:expr) => {{
        if $result.matches.len() >= $opts.max_count {
            return $result;
        }
        let m = create_match(&mut $lines, $opts, $lineno);
        $result.matches.push(m);
        if $opts.only_files.is_some() {
            return $result;
        }
    }};
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
            if regex.is_match(buf) {
                // found a match: create a dummy match object, and
                // leave it there (we never need more info than
                // "matched" or "didn't match")
                result.matches.push(Match::new(0, "".into(), Vec::new()));
            }
        }
    } else {
        let mut lines = Lines::new(buf);
        let mut match_offset = 0;
        let mut matched_lineno = !0_usize;  // let's say this is an invalid line number

        while let Some((mut start, mut end)) = regex.find(&buf[match_offset..]) {
            // back to offsets into buf
            start += match_offset;
            end += match_offset;

            // find the line numbers of the match
            let lineno = lines.get_lineno(start);
            let lineno_end = lines.get_lineno(end);
            if lineno != lineno_end {
                // match spans multiple lines: ignore it and start at the
                // beginning of the next line
                match_offset = lines.get_offset(lineno + 1);
                continue;
            } else if start == end {
                // are we at the end of the text?
                if start == buf.len() {
                    break;
                }
                // zero-size match: match this line and go to next
                match_offset = lines.get_offset(lineno + 1);
            } else {
                // start next match where this one ended
                match_offset = end;
            }

            if opts.invert {
                if lineno != matched_lineno {
                    // create matches for all inbetween lines:
                    // - matched_lineno is the last one with a match
                    // - lineno is the one with this match
                    for inb_lineno in matched_lineno.wrapping_add(1)..lineno {
                        new_match!(result, lines, opts, inb_lineno);
                    }
                    matched_lineno = lineno;
                }
            } else {
                // we have a new matching line?
                if lineno != matched_lineno {
                    new_match!(result, lines, opts, lineno);
                    matched_lineno = lineno;
                }
                // add this span to the match for this line
                if let Some(ref mut m) = result.matches.last_mut() {
                    let line_offset = lines.get_offset(lineno);
                    m.spans.push((start - line_offset, end - line_offset));
                }
            }
        }
        if opts.invert {
            // create matches for final lines
            for inb_lineno in matched_lineno.wrapping_add(1)..lines.get_lineno(buf.len())+1 {
                new_match!(result, lines, opts, inb_lineno);
            }
        }
    }
    result
}
