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

/// Helper: print a line with matched spans highlighted.
fn print_line_with_spans(m: &Match, colors: &Colors) {
    let mut pos = 0;
    for &(start, end) in &m.spans {
        if start > pos {
            print!("{}", &m.line[pos..start]);
        }
        print!("{}{}{}", colors.span, &m.line[start..end], colors.reset);
        pos = end;
    }
    println!("{}", &m.line[pos..]);
}

/// The default mode, used when printing to tty stdout.
///
/// Uses grouping by file names by default and can use colors.  Can print context.
#[derive(Clone)]
pub struct DefaultMode {
    colors: Colors,
    grouping: bool,
    is_first: bool,
}

impl DefaultMode {
    pub fn new(colors: Colors, grouping: bool) -> DefaultMode {
        DefaultMode {
            colors: colors,
            grouping: grouping,
            is_first: true,
        }
    }
}

impl DisplayMode for DefaultMode {
    fn print_result(&mut self, res: FileResult) {
        if res.matches.is_empty() {
            return;
        }
        if !self.is_first && self.grouping {
            println!("");
        }
        if res.is_binary {
            println!("Binary file {} matches.", res.fname);
        } else if res.has_context {
            // XXX refactor this mess!
            println!("{}{}{}", self.colors.path, res.fname, self.colors.reset);
            let mut last_printed_line = 0;
            for (im, m) in res.matches.iter().enumerate() {
                for (i, line) in m.before.iter().enumerate() {
                    let lno = m.lineno - m.before.len() + i;
                    if last_printed_line > 0 && lno > last_printed_line + 1 {
                        println!("{}--{}", self.colors.punct, self.colors.reset);
                    }
                    if lno > last_printed_line {
                        println!("{}{}{}{}-{}{}",
                                 self.colors.lineno, lno, self.colors.reset,
                                 self.colors.punct, self.colors.reset,
                                 line);
                        last_printed_line = lno;
                    }
                }
                if last_printed_line > 0 && m.lineno > last_printed_line + 1 {
                    println!("{}--{}", self.colors.punct, self.colors.reset);
                }
                print!("{}{}{}{}:{}",
                       self.colors.lineno, m.lineno, self.colors.reset,
                       self.colors.punct, self.colors.reset);
                print_line_with_spans(&m, &self.colors);
                last_printed_line = m.lineno;
                let next_match_line = if im < res.matches.len() - 1 {
                    res.matches[im + 1].lineno
                } else {
                    usize::MAX
                };
                for (i, line) in m.after.iter().enumerate() {
                    let lno = m.lineno + i + 1;
                    if lno < next_match_line {
                        println!("{}{}{}{}-{}{}",
                                 self.colors.lineno, lno, self.colors.reset,
                                 self.colors.punct, self.colors.reset,
                                 line);
                        last_printed_line = lno;
                    }
                }
            }
        } else {
            println!("{}{}{}", self.colors.path, res.fname, self.colors.reset);
            for m in res.matches {
                print!("{}{}{}{}:{}",
                       self.colors.lineno, m.lineno, self.colors.reset,
                       self.colors.punct, self.colors.reset);
                print_line_with_spans(&m, &self.colors);
            }
        }
        self.is_first = false;
    }
}

/// The one-match-per-line mode, used by defaultwhen printing to non-tty stdout.
///
/// Uses no grouping by default, but can use colors (for tty stdout with --nogroup).
/// Can print context.
#[derive(Clone)]
pub struct OneLineMode {
    colors: Colors,
    grouping: bool,
    is_first: bool,
}

impl OneLineMode {
    pub fn new(colors: Colors, grouping: bool) -> OneLineMode {
        OneLineMode {
            colors: colors,
            grouping: grouping,
            is_first: true,
        }
    }
}

impl DisplayMode for OneLineMode {
    fn print_result(&mut self, res: FileResult) {
        if res.matches.is_empty() {
            return;
        }
        if !self.is_first && self.grouping {
            println!("");
        }
        if res.is_binary {
            println!("Binary file {} matches.", res.fname);
        } else if res.has_context {
            // XXX refactor this copied mess!
            let mut last_printed_line = 0;
            for (im, m) in res.matches.iter().enumerate() {
                for (i, line) in m.before.iter().enumerate() {
                    let lno = m.lineno - m.before.len() + i;
                    if last_printed_line > 0 && lno > last_printed_line + 1 {
                        println!("{}--{}", self.colors.punct, self.colors.reset);
                    }
                    if lno > last_printed_line {
                        println!("{}{}{}{}-{}{}{}{}{}-{}{}",
                                 self.colors.path, res.fname, self.colors.reset,
                                 self.colors.punct, self.colors.reset,
                                 self.colors.lineno, lno, self.colors.reset,
                                 self.colors.punct, self.colors.reset,
                                 line);
                        last_printed_line = lno;
                    }
                }
                if last_printed_line > 0 && m.lineno > last_printed_line + 1 {
                    println!("{}--{}", self.colors.punct, self.colors.reset);
                }
                print!("{}{}{}{}:{}{}{}{}{}:{}",
                       self.colors.path, res.fname, self.colors.reset,
                       self.colors.punct, self.colors.reset,
                       self.colors.lineno, m.lineno, self.colors.reset,
                       self.colors.punct, self.colors.reset);
                print_line_with_spans(&m, &self.colors);
                last_printed_line = m.lineno;
                let next_match_line = if im < res.matches.len() - 1 {
                    res.matches[im + 1].lineno
                } else {
                    usize::MAX
                };
                for (i, line) in m.after.iter().enumerate() {
                    let lno = m.lineno + i + 1;
                    if lno < next_match_line {
                        println!("{}{}{}{}-{}{}{}{}{}-{}{}",
                                 self.colors.path, res.fname, self.colors.reset,
                                 self.colors.punct, self.colors.reset,
                                 self.colors.lineno, lno, self.colors.reset,
                                 self.colors.punct, self.colors.reset,
                                 line);
                        last_printed_line = lno;
                    }
                }
            }
        } else {
            for m in res.matches {
                print!("{}{}{}{}:{}{}{}{}{}:{}",
                       self.colors.path, res.fname, self.colors.reset,
                       self.colors.punct, self.colors.reset,
                       self.colors.lineno, m.lineno, self.colors.reset,
                       self.colors.punct, self.colors.reset);
                print_line_with_spans(&m, &self.colors);
            }
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
