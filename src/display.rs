// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

use std::io::{stdout, Write, Stdout};
use std::usize;

use search::{FileResult, Match};
use options::Colors;


/// A trait for printing search results to stdout.
pub trait DisplayMode {
    /// Print results from a single file.
    fn print_result(&mut self, res: FileResult);
}

/// The default mode, used when printing to tty stdout.
///
/// Uses grouping by file names by default and can use colors.  Can print context.
pub struct DefaultMode {
    colors: Colors,
    grouping: bool,
    heading: bool,
    is_first: bool,
    out: Stdout,
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
            out: stdout(),
        }
    }

    fn print_separator(&mut self) {
        w!(self.out, &self.colors.punct, b"--", &self.colors.reset, b"\n");
    }

    /// Helper: print a line with matched spans highlighted.
    fn print_line_with_spans(&mut self, m: &Match) {
        if self.colors.empty {
            w!(self.out, &m.line, b"\n");
        } else {
            let mut pos = 0;
            for &(start, end) in &m.spans {
                if start > pos {
                    w!(self.out, &m.line[pos..start]);
                }
                w!(self.out, &self.colors.span, &m.line[start..end], &self.colors.reset);
                pos = end;
            }
            w!(self.out, &m.line[pos..], b"\n");
        }
    }

    /// Helper: print a match with custom callbacks for file header and match line.
    fn match_printer<FF, LF>(&mut self, res: &FileResult, file_func: FF, line_func: LF)
        where FF: Fn(&mut Self, &FileResult), LF: Fn(&mut Self, &FileResult, usize, &'static [u8])
    {
        // (maybe) print a heading for the whole file
        file_func(self, &res);
        // easy case without context lines
        if !res.has_context {
            for m in &res.matches {
                line_func(self, res, m.lineno, b":");
                self.print_line_with_spans(&m);
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
                    self.print_separator();
                }
                // only print this line if we didn't print it before, e.g.
                // as a match line or after-context line
                if lno > last_printed_line {
                    line_func(self, res, lno, b"-");
                    w!(self.out, &line, b"\n");
                    last_printed_line = lno;
                }
            }
            if last_printed_line > 0 && m.lineno > last_printed_line + 1 {
                self.print_separator();
            }
            line_func(self, res, m.lineno, b":");
            self.print_line_with_spans(&m);
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
                line_func(self, res, lno, b"-");
                w!(self.out, &line, b"\n");
                last_printed_line = lno;
            }
        }
    }
}

impl DisplayMode for DefaultMode {

    fn print_result(&mut self, res: FileResult) {
        // files with no matches never print anything
        if res.matches.is_empty() {
            return;
        }
        // grouping separator, but not on the first file
        if !self.is_first && self.grouping {
            w!(self.out, b"\n");
            if res.has_context && !self.heading {
                // in context mode, we have to print a "--" separator between files
                self.print_separator();
            }
        }
        if res.is_binary {
            // special message for binary files
            w!(self.out, b"Binary file ", res.fname.as_bytes(), b" matches.\n");
        } else if self.heading {
            // headings mode: print file name first, then omit it from match lines
            self.match_printer(&res, |slf, res| {
                w!(slf.out,
                   &slf.colors.path, res.fname.as_bytes(), &slf.colors.reset, b"\n");
            }, |slf, _, lineno, sep| {
                w!(slf.out,
                   &slf.colors.lineno, format!("{}", lineno).as_bytes(), &slf.colors.reset,
                   &slf.colors.punct, sep, &slf.colors.reset);
            });
        } else {
            // no headings mode: print file name on every match line
            self.match_printer(&res, |_, _| { }, |slf, res, lineno, sep| {
                w!(slf.out,
                   &slf.colors.path, res.fname.as_bytes(), &slf.colors.reset,
                   &slf.colors.punct, sep, &slf.colors.reset,
                   &slf.colors.lineno, format!("{}", lineno).as_bytes(), &slf.colors.reset,
                   &slf.colors.punct, sep, &slf.colors.reset);
            });
        }
        self.is_first = false;
    }
}

/// The mode used for --ackmate mode.
///
/// No colors, one matched line per line, all spans indicated numerically.
pub struct AckMateMode {
    is_first: bool,
    out: Stdout,
}

impl AckMateMode {
    pub fn new() -> AckMateMode {
        AckMateMode {
            is_first: true,
            out: stdout(),
        }
    }
}

impl DisplayMode for AckMateMode {
    fn print_result(&mut self, res: FileResult) {
        if res.matches.is_empty() {
            return;
        }
        if !self.is_first {
            w!(self.out, b"\n");
        }
        if res.is_binary {
            w!(self.out, b"Binary file ", res.fname.as_bytes(), b" matches.\n");
        } else {
            w!(self.out, b":", res.fname.as_bytes());
            for m in res.matches {
                let spans = m.spans.iter()
                                   .map(|&(s, e)| format!("{} {}", s, e - s))
                                   .collect::<Vec<_>>().join(",");
                w!(self.out,
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
pub struct VimGrepMode {
    out: Stdout
}

impl VimGrepMode {
    pub fn new() -> Self {
        VimGrepMode {
            out: stdout(),
        }
    }
}

impl DisplayMode for VimGrepMode {
    fn print_result(&mut self, res: FileResult) {
        if res.matches.is_empty() {
            return;
        }
        if res.is_binary {
            println!("Binary file {} matches.", res.fname);
        } else {
            for m in res.matches {
                for s in &m.spans {
                    w!(self.out,
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
pub struct FilesOnlyMode {
    colors: Colors,
    need_match: bool,
    out: Stdout,
}

impl FilesOnlyMode {
    pub fn new(colors: Colors, need_match: bool) -> FilesOnlyMode {
        FilesOnlyMode {
            colors: colors,
            need_match: need_match,
            out: stdout(),
        }
    }
}

impl DisplayMode for FilesOnlyMode {
    fn print_result(&mut self, res: FileResult) {
        if res.matches.is_empty() != self.need_match {
            w!(self.out, &self.colors.path, &res.fname.as_bytes(), &self.colors.reset, b"\n");
        }
    }
}

/// The mode used for --count mode.
///
/// One file per line, followed by match count (not matched line count).
pub struct CountMode {
    colors: Colors,
    out: Stdout,
}

impl CountMode {
    pub fn new(colors: Colors) -> CountMode {
        CountMode {
            colors: colors,
            out: stdout(),
        }
    }
}

impl DisplayMode for CountMode {
    fn print_result(&mut self, res: FileResult) {
        if res.matches.is_empty() {
            return;
        }
        let count: usize = res.matches.iter().map(|m| m.spans.iter().count())
                                             .fold(0, |a, v| a + v);
        w!(self.out,
           &self.colors.path, &res.fname.as_bytes(), &self.colors.reset,
           &self.colors.punct, b":", &self.colors.reset,
           &self.colors.lineno, &format!("{}", count).as_bytes(), &self.colors.reset,
           b"\n");
    }
}
