// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

use std::cmp::min;
use std::usize;

use libc;
use clap::{App, AppSettings, Arg};
use num_cpus;

/// Contains the ANSI codes needed to set the terminal to a certain color.
#[derive(Clone)]
pub struct Colors {
    pub reset: String,
    pub path: String,
    pub lineno: String,
    pub span: String,
    pub punct: String,
    pub empty: bool,
}

impl Colors {
    /// Create a struct where no colors are emitted.
    fn empty() -> Colors {
        Colors {
            reset: "".into(),
            path: "".into(),
            lineno: "".into(),
            span: "".into(),
            punct: "".into(),
            empty: true,
        }
    }

    /// Create a struct from given color specs.  Color specs are the payload
    /// of the color ANSI sequences, e.g. "01;31".
    fn from(path: &str, lineno: &str, span: &str, punct: &str) -> Colors {
        Colors {
            reset: "\x1b[0m".into(),
            path: format!("\x1b[{}m", path),
            lineno: format!("\x1b[{}m", lineno),
            span: format!("\x1b[{}m", span),
            punct: format!("\x1b[{}m", punct),
            empty: false,
        }
    }
}

/// Case-sensitivity matching options.
///
/// Smart casing means insensitive as long as the pattern contains no uppercase
/// letters.
#[derive(Clone)]
pub enum Casing {
    Default,
    Smart,
    Insensitive,
}

/// Holds all options for the search.
#[derive(Clone)]
pub struct Opts {
    // file related options
    pub path: String,
    pub depth: usize,
    pub follow_links: bool,
    pub do_binaries: bool,
    pub do_hidden: bool,
    // ignore file related options
    pub check_ignores: bool,
    // pattern related options
    pub pattern: String,
    pub casing: Casing,
    pub literal: bool,
    pub invert: bool,
    // display related options
    pub colors: Option<Colors>,
    pub only_files: Option<bool>,
    pub only_count: bool,
    pub show_break: bool,
    pub show_heading: bool,
    pub ackmate_format: bool,
    pub vimgrep_format: bool,
    pub max_count: usize,
    pub before: usize,
    pub after: usize,
    // others
    pub workers: u32,
}

// Taken from libtest, there seems to be no better way presently.
// There are a few libraries on crates.io, but without Windows support.

#[cfg(unix)]
fn stdout_isatty() -> bool {
    unsafe { libc::isatty(libc::STDOUT_FILENO) != 0 }
}
#[cfg(windows)]
fn stdout_isatty() -> bool {
    const STD_OUTPUT_HANDLE: libc::DWORD = -11i32 as libc::DWORD;
    extern "system" {
        fn GetStdHandle(which: libc::DWORD) -> libc::HANDLE;
        fn GetConsoleMode(hConsoleHandle: libc::HANDLE,
                          lpMode: libc::LPDWORD) -> libc::BOOL;
    }
    unsafe {
        let handle = GetStdHandle(STD_OUTPUT_HANDLE);
        let mut out = 0;
        GetConsoleMode(handle, &mut out) != 0
    }
}

/// Somewhat simpler creation of flag Args.
macro_rules! flag {
    ($n:ident -$f:ident) => {
        Arg::with_name(stringify!($n)).short(stringify!($f))
    };
    ($n:ident -$f:ident --$l:expr) => {
        Arg::with_name(stringify!($n)).short(stringify!($f)).long($l)
    };
    ($n:ident / --$l:expr) => {
        Arg::with_name(stringify!($n)).long($l)
    };
}

impl Opts {
    pub fn from_cmdline() -> Opts {
        let version = format!("v{}", crate_version!());
        // XXX: sort and group the arguments once they are all done
        let app = App::new("Ruthenium")
            .version(&version)
            .usage("ru [options] PATTERN [PATH]")
            .about("Recursively search for a pattern, like ack")
            .setting(AppSettings::UnifiedHelpMessage)
            .setting(AppSettings::ArgRequiredElseHelp)  // seems to be not working
            .arg(Arg::with_name("pattern").required(true).index(1))
            .arg(Arg::with_name("path").index(2))
            .arg(flag!(all -a --"all-types"))
            .arg(flag!(depth / --"depth").takes_value(true))
            .arg(flag!(literal -Q --"literal"))
            .arg(flag!(fixedstrings -F --"fixed-strings"))
            .arg(flag!(alltext -t --"all-text").conflicts_with("all"))
            .arg(flag!(unrestricted -u --"unrestricted").conflicts_with("all"))
            .arg(flag!(searchbinary / --"search-binary"))
            .arg(flag!(searchhidden / --"hidden"))
            .arg(flag!(fileswith -l --"files-with-matches"))
            .arg(flag!(fileswithout -L --"files-without-matches").conflicts_with("fileswith"))
            .arg(flag!(count -c --"count").conflicts_with("fileswith"))
            .arg(flag!(follow -f --"follow"))
            .arg(flag!(nofollow / --"nofollow").conflicts_with("follow"))
            .arg(flag!(nocolor / --"nocolor"))
            .arg(flag!(colorlineno / --"color-line-number").takes_value(true))
            .arg(flag!(colorspan / --"color-match").takes_value(true))
            .arg(flag!(colorpath / --"color-path").takes_value(true))
            .arg(flag!(colorpunct / --"color-punct").takes_value(true))
            .arg(flag!(casesens -s --"case-sensitive").conflicts_with("caseinsens"))
            .arg(flag!(casesmart -S --"smart-case").conflicts_with("casesens"))
            .arg(flag!(caseinsens -i --"ignore-case").conflicts_with("casesmart"))
            .arg(flag!(group / --"group"))
            .arg(flag!(nogroup / --"nogroup").conflicts_with("gorup"))
            .arg(flag!(heading -H --"heading"))
            .arg(flag!(noheading / --"noheading").conflicts_with("heading"))
            .arg(flag!(break / --"break"))
            .arg(flag!(nobreak / --"nobreak").conflicts_with("break"))
            .arg(flag!(ackmate / --"ackmate"))
            .arg(flag!(vimgrep / --"vimgrep"))
            .arg(flag!(maxcount -m --"max-count").takes_value(true))
            .arg(flag!(before -B --"before").takes_value(true))
            .arg(flag!(after -A --"after").takes_value(true))
            .arg(flag!(context -C --"context").takes_value(true))
            .arg(flag!(workers / --"workers").takes_value(true))
            .arg(flag!(invert -v --"invert-match"))
            ;
        let m = app.get_matches();

        // process option values
        let depth = m.value_of("depth").and_then(|v| v.parse::<usize>().ok())
                                       .map(|v| v + 1) // 0 == immediate children
                                       .unwrap_or(usize::MAX);

        let mut binaries = m.is_present("searchbinary");
        let mut hidden = m.is_present("searchhidden");
        let mut ignores = true;
        if m.is_present("all") {
            binaries = true;
            ignores = false;
        } else if m.is_present("alltext") {
            ignores = false;
        } else if m.is_present("unrestricted") {
            binaries = true;
            hidden = true;
            ignores = false;
        }

        let mut casing = Casing::Smart;
        if m.is_present("caseinsens") {
            casing = Casing::Insensitive;
        } else if m.is_present("casesens") {
            casing = Casing::Default;
        }
        let mut literal = m.is_present("literal");
        if m.is_present("fixedstrings") {
            literal = true;
        }

        let out_to_tty = stdout_isatty();
        let colors = if !out_to_tty || m.is_present("nocolor") {
            Colors::empty()
        } else {
            Colors::from(
                m.value_of("colorpath").unwrap_or("35"),
                m.value_of("colorlineno").unwrap_or("32"),
                m.value_of("colorspan").unwrap_or("4"),
                m.value_of("colorpunct").unwrap_or("36"),
            )
        };
        let mut heading = out_to_tty;
        let mut showbreak = out_to_tty;
        if m.is_present("heading") {
            heading = true;
        } else if m.is_present("noheading") {
            heading = false;
        }
        if m.is_present("break") {
            showbreak = true;
        } else if m.is_present("nobreak") {
            showbreak = false;
        }
        if m.is_present("group") {
            heading = true;
            showbreak = true;
        } else if m.is_present("nogroup") {
            heading = false;
            showbreak = false;
        }
        let maxcount = m.value_of("maxcount").and_then(|v| v.parse().ok())
                                             .unwrap_or(usize::MAX);
        let mut before = m.value_of("before").and_then(|v| v.parse().ok())
                                             .unwrap_or(0);
        let mut after = m.value_of("after").and_then(|v| v.parse().ok())
                                           .unwrap_or(0);
        if m.is_present("context") {
            before = m.value_of("context").unwrap().parse().ok().unwrap_or(0);
            after = before;
        }

        let workers = m.value_of("workers").and_then(|v| v.parse().ok())
                                           .unwrap_or(min(4, num_cpus::get())) as u32;

        Opts {
            // file related
            path: m.value_of("path").unwrap_or(".").into(),
            depth: depth,
            follow_links: m.is_present("follow"),
            do_binaries: binaries,
            do_hidden: hidden,
            // ignore file related
            check_ignores: ignores,
            // pattern related
            pattern: m.value_of("pattern").unwrap().into(),
            casing: casing,
            literal: literal,
            invert: m.is_present("invert"),
            // display related
            colors: Some(colors),
            only_files: if m.is_present("fileswith") {
                Some(true)
            } else if m.is_present("fileswithout") {
                Some(false)
            } else { None },
            only_count: m.is_present("count"),
            show_break: showbreak,
            show_heading: heading,
            ackmate_format: m.is_present("ackmate"),
            vimgrep_format: m.is_present("vimgrep"),
            max_count: maxcount,
            before: before,
            after: after,
            // other
            workers: workers,
        }
    }
}
