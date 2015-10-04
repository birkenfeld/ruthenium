// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

use search::{FileResult, Match};
use options::Colors;


pub trait DisplayMode: Send + Clone + 'static {
    fn print_result(&mut self, res: FileResult);
}

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
        if res.matches.is_empty() == !self.need_match {
            println!("{}{}{}", self.colors.path, res.fname, self.colors.reset);
        }
    }
}

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
