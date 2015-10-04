// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

use clap::{App, Arg};

#[derive(Clone)]
pub struct Colors {
    pub path: String,
    pub lineno: String,
    pub span: String,
}

#[derive(Clone)]
pub struct Opts {
    pub pattern: String,
    pub path: String,
    pub depth: usize,
    pub follow_links: bool,
    pub literal: bool,
    pub do_binaries: bool,
    pub do_hidden: bool,
    pub check_ignores: bool,
    pub only_files: Option<bool>,
}

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
        let app = App::new("Ruthenium")
            .version(&version)
            .usage("ru [options] PATTERN [PATH]")
            .about("Recursively search for a pattern, like ack")
            .arg(Arg::with_name("pattern").required(true).index(1))
            .arg(Arg::with_name("path").index(2))
            .arg(flag!(all -a --"all-types"))
            .arg(flag!(depth / --"depth").takes_value(true))
            .arg(flag!(literal -Q --"literal"))
            .arg(flag!(fixedstrings -F --"fixed-strings"))
            .arg(flag!(alltext -t --"all-text").conflicts_with("all"))
            .arg(flag!(unrestricted -u --"unrestricted").conflicts_with("all"))
            .arg(flag!(searchbinary / --"searchbinary"))
            .arg(flag!(searchhidden / --"hidden"))
            .arg(flag!(fileswith -l --"files-with-matches"))
            .arg(flag!(fileswithout -L --"files-without-matches").conflicts_with("fileswith"))
            .arg(flag!(follow -f --"follow"))
            ;
        let m = app.get_matches();
        let mut binaries = m.is_present("searchbinary");
        let mut hidden = m.is_present("searchhidden");
        let mut ignores = true;
        let mut literal = m.is_present("literal");
        if m.is_present("fixedstrings") {
            literal = true;
        }
        if m.is_present("all") {
            binaries = true;
            ignores = false;
        }
        else if m.is_present("alltext") {
            ignores = false;
        }
        else if m.is_present("unrestricted") {
            binaries = true;
            hidden = true;
            ignores = false;
        }
        Opts {
            pattern: m.value_of("pattern").unwrap().into(),
            path: m.value_of("path").unwrap_or(".").into(),
            depth: m.value_of("depth").and_then(|v| v.parse().ok()).unwrap_or(::std::usize::MAX),
            follow_links: m.is_present("follow"),
            literal: literal,
            do_binaries: binaries,
            do_hidden: hidden,
            check_ignores: ignores,
            only_files: if m.is_present("fileswith") {
                Some(true)
            } else if m.is_present("fileswithout") {
                Some(false)
            } else { None },
        }
    }
}
