// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

#[derive(Clone)]
pub struct Opts {
    pub pattern: String,
    pub path: String,
    pub depth: usize,
    pub follow_links: bool,
    pub do_binaries: bool,
    pub do_hidden: bool,
    pub check_ignores: bool,
    pub only_files: Option<bool>,
}
