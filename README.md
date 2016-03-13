# Ruthenium, an Ack-like searcher

Ruthenium is an attempt to implement the well-known Perl tool `ack` in Rust.

When finished, it is supposed to show the strengths of Rust, for example simple
and efficient concurrency without locks, and speed comparable with C programs,
in this case the implementation called `ag` or `the_silver_searcher`.

## How to build

Use `cargo build --release`.  `target/release/ru` is the binary.

## How to use

The resulting binary is linked statically against Rust dependencies, so it can
be copied into a `bin` directory and used.

### Command line

Command-line options are designed to be mostly compatible with Ag.  There are
probably small differences, especially in the handling of ignore files.

### Regex engines

Currently, the regex engine can be selected to be either Andrew Gallant's Rust
implementation `regex` (the default) or PCRE (requires libpcre and its headers
to be installed).  Select the latter with the Cargo feature flag `pcre`.
