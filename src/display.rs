// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

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

impl DefaultMode {
    pub fn new(colors: Colors, grouping: bool, heading: bool) -> DefaultMode {
        DefaultMode {
            colors: colors,
            grouping: grouping,
            heading: heading,
            is_first: true,
        }
    }

    fn print_separator(&self) {
        println!("{}--{}", self.colors.punct, self.colors.reset);
    }

    /// Helper: print a line with matched spans highlighted.
    fn print_line_with_spans(&self, m: &Match) {
        let mut pos = 0;
        for &(start, end) in &m.spans {
            if start > pos {
                print!("{}", &m.line[pos..start]);
            }
            print!("{}{}{}",
                   self.colors.span, &m.line[start..end], self.colors.reset);
            pos = end;
        }
        println!("{}", &m.line[pos..]);
    }

    /// Helper: print a match with custom callbacks for file header and match line.
    fn match_printer<FF, LF>(&self, res: &FileResult, file_func: FF, line_func: LF)
        where FF: Fn(&FileResult), LF: Fn(&FileResult, usize, &'static str)
    {
        // (maybe) print a heading for the whole file
        file_func(&res);
        // easy case without context lines
        if !res.has_context {
            for m in &res.matches {
                line_func(res, m.lineno, ":");
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
                    line_func(res, lno, "-");
                    println!("{}", line);
                    last_printed_line = lno;
                }
            }
            if last_printed_line > 0 && m.lineno > last_printed_line + 1 {
                self.print_separator();
            }
            line_func(res, m.lineno, ":");
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
                line_func(res, lno, "-");
                println!("{}", line);
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
            println!("");
            if res.has_context && !self.heading {
                // in context mode, we have to print a "--" separator between files
                self.print_separator();
            }
        }
        if res.is_binary {
            // special message for binary files
            println!("Binary file {} matches.", res.fname);
        } else if self.heading {
            // headings mode: print file name first, then omit it from match lines
            self.match_printer(&res, |res| {
                println!("{}{}{}", self.colors.path, res.fname, self.colors.reset);
            }, |_, lineno, sep| {
                print!("{}{}{}{}{}{}",
                       self.colors.lineno, lineno, self.colors.reset,
                       self.colors.punct, sep, self.colors.reset);
            });
        } else {
            // no headings mode: print file name on every match line
            self.match_printer(&res, |_| { }, |res, lineno, sep| {
                print!("{}{}{}{}{}{}{}{}{}{}{}{}",
                       self.colors.path, res.fname, self.colors.reset,
                       self.colors.punct, sep, self.colors.reset,
                       self.colors.lineno, lineno, self.colors.reset,
                       self.colors.punct, sep, self.colors.reset);
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
        if res.matches.is_empty() {
            return;
        }
        if !self.is_first {
            println!("");
        }
        if res.is_binary {
            println!("Binary file {} matches.", res.fname);
        } else {
            println!(":{}", res.fname);
            for m in res.matches {
                let spans = m.spans.iter()
                                   .map(|&(s, e)| format!("{} {}", s, e - s))
                                   .collect::<Vec<_>>().join(",");
                println!("{};{}:{}", m.lineno, spans, m.line);
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
        if res.matches.is_empty() {
            return;
        }
        if res.is_binary {
            println!("Binary file {} matches.", res.fname);
        } else {
            for m in res.matches {
                for s in &m.spans {
                    println!("{}:{}:{}:{}", res.fname, m.lineno, s.0 + 1, m.line);
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
        if res.matches.is_empty() != self.need_match {
            println!("{}{}{}", self.colors.path, res.fname, self.colors.reset);
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
        if res.matches.is_empty() {
            return;
        }
        let count: usize = res.matches.iter().map(|m| m.spans.iter().count())
                                             .fold(0, |a, v| a + v);
        println!("{}{}{}{}:{}{}{}{}",
                 self.colors.path, res.fname, self.colors.reset,
                 self.colors.punct, self.colors.reset,
                 self.colors.lineno, count, self.colors.reset);
    }
}
