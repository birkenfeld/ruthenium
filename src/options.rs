// ---------------------------------------------------------------------------------------
// Ruthenium, an ack-like searcher, (c) 2015 Georg Brandl.
// Licensed under the MIT license.
// ---------------------------------------------------------------------------------------

pub struct Opts {
    pub pattern: String,
    pub path: String,
    pub all_files: bool,
    pub only_files: Option<bool>,
}
