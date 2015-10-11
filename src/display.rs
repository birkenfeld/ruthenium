// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

use std::cell::RefCell;
use std::io::{stdout, Write, Stdout};
use std::usize;

use search::{FileResult, Match};
use options::Colors;


/// A trait for printing search results to stdout.
pub trait DisplayMode: Send + Clone + 'static {
    /// Print results from a single file.
    fn print_result(&mut self, res: FileResult);
}

/// The default mode, used when printing to tty stdout.
///
/// Uses grouping by file names by default and can use colors.  Can print context.
#[derive(Clone)]
pub struct DefaultMode {
    colors: Colors,
    grouping: bool,
    heading: bool,
    is_first: bool,
}

macro_rules! w {
    ($out:expr, $first:expr, $($rest:expr),*) => {
        let _ = $out.write($first);
        w!($out, $($rest),*);
    };
    ($out:expr, $first:expr) => {
        let _ = $out.write($first);
    }
}

impl DefaultMode {
    pub fn new(colors: Colors, grouping: bool, heading: bool) -> DefaultMode {
        DefaultMode {
            colors: colors,
            grouping: grouping,
            heading: heading,
            is_first: true,
        }
    }

    fn print_separator(&self, out: &RefCell<Stdout>) {
        w!(out.borrow_mut(), &self.colors.punct, b"--", &self.colors.reset, b"\n");
    }

    /// Helper: print a line with matched spans highlighted.
    fn print_line_with_spans(&self, m: &Match, out: &RefCell<Stdout>) {
        if self.colors.empty {
            w!(out.borrow_mut(), &m.line, b"\n");
        } else {
            let mut pos = 0;
            for &(start, end) in &m.spans {
                if start > pos {
                    w!(out.borrow_mut(), &m.line[pos..start]);
                }
                w!(out.borrow_mut(), &self.colors.span, &m.line[start..end], &self.colors.reset);
                pos = end;
            }
            w!(out.borrow_mut(), &m.line[pos..], b"\n");
        }
    }

    /// Helper: print a match with custom callbacks for file header and match line.
    fn match_printer<FF, LF>(&self, res: &FileResult, mut out: &RefCell<Stdout>,
                             file_func: FF, line_func: LF)
        where FF: Fn(&FileResult), LF: Fn(&FileResult, usize, &'static [u8])
    {
        // (maybe) print a heading for the whole file
        file_func(&res);
        // easy case without context lines
        if !res.has_context {
            for m in &res.matches {
                line_func(res, m.lineno, b":");
                self.print_line_with_spans(&m, &mut out);
            }
            return;
        }
        // remember the last printed line: to be able to print "--" separators
        // between non-consecutive lines in context mode
        let mut last_printed_line = 0;
        for (im, m) in res.matches.iter().enumerate() {
            // print before-context
            for (i, line) in m.before.iter().enumerate() {
                let lno = m.lineno - m.before.len() + i;
                if last_printed_line > 0 && lno > last_printed_line + 1 {
                    self.print_separator(&mut out);
                }
                // only print this line if we didn't print it before, e.g.
                // as a match line or after-context line
                if lno > last_printed_line {
                    line_func(res, lno, b"-");
                    w!(out.borrow_mut(), &line, b"\n");
                    last_printed_line = lno;
                }
            }
            if last_printed_line > 0 && m.lineno > last_printed_line + 1 {
                self.print_separator(&mut out);
            }
            line_func(res, m.lineno, b":");
            self.print_line_with_spans(&m, &mut out);
            // print after-context
            last_printed_line = m.lineno;
            // determine line number of next match, since we have to stop
            // printing context *before* that line
            let next_match_line = if im < res.matches.len() - 1 {
                res.matches[im + 1].lineno
            } else {
                usize::MAX
            };
            for (i, line) in m.after.iter().enumerate() {
                let lno = m.lineno + i + 1;
                // stop when we hit the next match
                if lno >= next_match_line {
                    break;
                }
                line_func(res, lno, b"-");
                w!(out.borrow_mut(), &line, b"\n");
                last_printed_line = lno;
            }
        }
    }
}

impl DisplayMode for DefaultMode {

    fn print_result(&mut self, res: FileResult) {
        let out = RefCell::new(stdout());
        // files with no matches never print anything
        if res.matches.is_empty() {
            return;
        }
        // grouping separator, but not on the first file
        if !self.is_first && self.grouping {
            w!(out.borrow_mut(), b"\n");
            if res.has_context && !self.heading {
                // in context mode, we have to print a "--" separator between files
                self.print_separator(&out);
            }
        }
        if res.is_binary {
            // special message for binary files
            w!(out.borrow_mut(), b"Binary file ", res.fname.as_bytes(), b" matches.\n");
        } else if self.heading {
            // headings mode: print file name first, then omit it from match lines
            self.match_printer(&res, &out, |res| {
                w!(out.borrow_mut(),
                   &self.colors.path, res.fname.as_bytes(), &self.colors.reset, b"\n");
            }, |_, lineno, sep| {
                w!(out.borrow_mut(),
                   &self.colors.lineno, format!("{}", lineno).as_bytes(), &self.colors.reset,
                   &self.colors.punct, sep, &self.colors.reset);
            });
        } else {
            // no headings mode: print file name on every match line
            self.match_printer(&res, &out, |_| { }, |res, lineno, sep| {
                w!(out.borrow_mut(),
                   &self.colors.path, res.fname.as_bytes(), &self.colors.reset,
                   &self.colors.punct, sep, &self.colors.reset,
                   &self.colors.lineno, format!("{}", lineno).as_bytes(), &self.colors.reset,
                   &self.colors.punct, sep, &self.colors.reset);
            });
        }
        self.is_first = false;
    }
}

/// The mode used for --ackmate mode.
///
/// No colors, one matched line per line, all spans indicated numerically.
#[derive(Clone)]
pub struct AckMateMode {
    is_first: bool,
}

impl AckMateMode {
    pub fn new() -> AckMateMode {
        AckMateMode {
            is_first: true,
        }
    }
}

impl DisplayMode for AckMateMode {
    fn print_result(&mut self, res: FileResult) {
        let mut out = stdout();
        if res.matches.is_empty() {
            return;
        }
        if !self.is_first {
            w!(out, b"\n");
        }
        if res.is_binary {
            w!(out, b"Binary file ", res.fname.as_bytes(), b" matches.\n");
        } else {
            w!(out, b":", res.fname.as_bytes());
            for m in res.matches {
                let spans = m.spans.iter()
                                   .map(|&(s, e)| format!("{} {}", s, e - s))
                                   .collect::<Vec<_>>().join(",");
                w!(out,
                   &format!("{};{}:", m.lineno, spans).as_bytes(),
                   &m.line, b"\n");
            }
        }
        self.is_first = false;
    }
}

/// The mode used for --vimgrep mode.
///
/// No colors, one match per line (so lines with multiple matches are printed
/// multiple times).
#[derive(Clone)]
pub struct VimGrepMode;

impl DisplayMode for VimGrepMode {
    fn print_result(&mut self, res: FileResult) {
        let mut out = stdout();
        if res.matches.is_empty() {
            return;
        }
        if res.is_binary {
            println!("Binary file {} matches.", res.fname);
        } else {
            for m in res.matches {
                for s in &m.spans {
                    w!(out,
                       &format!("{}:{}:{}:", res.fname, m.lineno, s.0 + 1).as_bytes(),
                       &m.line, b"\n");
                }
            }
        }
    }
}

/// The mode used for --files-with-matches and --files-without-matches.
///
/// One file per line, no contents printed.
#[derive(Clone)]
pub struct FilesOnlyMode {
    colors: Colors,
    need_match: bool,
}

impl FilesOnlyMode {
    pub fn new(colors: Colors, need_match: bool) -> FilesOnlyMode {
        FilesOnlyMode {
            colors: colors,
            need_match: need_match,
        }
    }
}

impl DisplayMode for FilesOnlyMode {
    fn print_result(&mut self, res: FileResult) {
        let mut out = stdout();
        if res.matches.is_empty() != self.need_match {
            w!(out, &self.colors.path, &res.fname.as_bytes(), &self.colors.reset, b"\n");
        }
    }
}

/// The mode used for --count mode.
///
/// One file per line, followed by match count (not matched line count).
#[derive(Clone)]
pub struct CountMode {
    colors: Colors,
}

impl CountMode {
    pub fn new(colors: Colors) -> CountMode {
        CountMode {
            colors: colors,
        }
    }
}

impl DisplayMode for CountMode {
    fn print_result(&mut self, res: FileResult) {
        let mut out = stdout();
        if res.matches.is_empty() {
            return;
        }
        let count: usize = res.matches.iter().map(|m| m.spans.iter().count())
                                             .fold(0, |a, v| a + v);
        w!(out,
           &self.colors.path, &res.fname.as_bytes(), &self.colors.reset,
           &self.colors.punct, b":", &self.colors.reset,
           &self.colors.lineno, &format!("{}", count).as_bytes(), &self.colors.reset,
           b"\n");
    }
}
