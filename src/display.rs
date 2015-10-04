// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

use search::FileResult;
use options::Colors;


pub trait DisplayMode: Send + Clone + 'static {
    fn print_result(&mut self, res: FileResult);
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
                       self.colors.lineno,
                       m.lineno,
                       self.colors.reset,
                       self.colors.punct,
                       self.colors.reset);
                let mut pos = 0;
                for (start, end) in m.spans {
                    if start > pos {
                        print!("{}", &m.line[pos..start]);
                    }
                    print!("{}{}{}", self.colors.span, &m.line[start..end], self.colors.reset);
                    pos = end;
                }
                println!("{}", &m.line[pos..]);
            }
        }
        self.is_first = false;
    }
}

#[derive(Clone)]
pub struct OneLineMode {
    colors: Option<Colors>,
}

impl OneLineMode {
    pub fn new(colors: Option<Colors>) -> OneLineMode {
        OneLineMode {
            colors: colors,
        }
    }
}

impl DisplayMode for OneLineMode {
    fn print_result(&mut self, res: FileResult) {
        if res.matches.is_empty() {
            return;
        }
        if res.is_binary {
            println!("Binary file {} matches.", res.fname);
        } else {
            for m in res.matches {
                println!("{}:{}:{}", res.fname, m.lineno, m.line);
            }
        }
    }
}

#[derive(Clone)]
pub struct FilesOnlyMode;

impl DisplayMode for FilesOnlyMode {
    fn print_result(&mut self, res: FileResult) {
        if !res.matches.is_empty() {
            println!("{}", res.fname);
        }
    }
}

#[derive(Clone)]
pub struct FilesWithoutMatchMode;

impl DisplayMode for FilesWithoutMatchMode {
    fn print_result(&mut self, res: FileResult) {
        if res.matches.is_empty() {
            println!("{}", res.fname);
        }
    }
}
