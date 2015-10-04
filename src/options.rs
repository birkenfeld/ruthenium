// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

use clap::{App, AppSettings, Arg};

#[derive(Clone)]
pub struct Colors {
    pub reset: String,
    pub path: String,
    pub lineno: String,
    pub span: String,
    pub punct: String,
}

impl Colors {
    fn empty() -> Colors {
        Colors {
            reset: "".into(),
            path: "".into(),
            lineno: "".into(),
            span: "".into(),
            punct: "".into(),
        }
    }

    fn from(path: &str, lineno: &str, span: &str, punct: &str) -> Colors {
        Colors {
            reset: "\x1b[0m".into(),
            path: format!("\x1b[{}m", path),
            lineno: format!("\x1b[{}m", lineno),
            span: format!("\x1b[{}m", span),
            punct: format!("\x1b[{}m", punct),
        }
    }
}

#[derive(Clone)]
pub enum Casing {
    Default,
    Smart,
    Insensitive,
}

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
    // display related options
    pub colors: Option<Colors>,
    pub only_files: Option<bool>,
    pub only_count: bool,
    pub show_break: bool,
    pub show_heading: bool,
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
            .setting(AppSettings::UnifiedHelpMessage)
            .setting(AppSettings::ArgRequiredElseHelp)
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
            ;
        let m = app.get_matches();

        let depth = m.value_of("depth").and_then(|v| v.parse::<usize>().ok())
                                       .map(|v| v + 1) // 0 == immediate children
                                       .unwrap_or(::std::usize::MAX);

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

        let colors = if m.is_present("nocolor") {
            Colors::empty()
        } else {
            Colors::from(
                m.value_of("colorpath").unwrap_or("35"),
                m.value_of("colorlineno").unwrap_or("32"),
                m.value_of("colorspan").unwrap_or("4"),
                m.value_of("colorpunct").unwrap_or("1;36"),
            )
        };
        let mut heading = true;
        let mut showbreak = true;
        if m.is_present("noheading") {
            heading = false;
        }
        if m.is_present("nobreak") {
            showbreak = false;
        }
        if m.is_present("nogroup") {
            heading = false;
            showbreak = false;
        }

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
        }
    }
}
