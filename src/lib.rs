//! Some doc
#![feature(rustc_private)]
#![allow(unused_imports, dead_code)]
extern crate syntax;

#[macro_use]
pub mod report;
mod utils;
mod compare;
mod cfg;

use std::path::Path;

use syntax::ast::{self, StmtKind, Item, ItemKind, Attribute, MetaItemKind, Visibility, Crate};

pub use utils::parse_crate;
pub use compare::compare_crates;
use utils::Indent;

pub fn create_report(old: &Path, new: &Path) -> report::Report {
    let old_crate = parse_crate(old);
    let new_crate = parse_crate(new);

    let mut report = report::Report::new();
    if old_crate.is_none() {
        push!(report, Error, "Failed to read crate at {}", old.display());
    }
    if new_crate.is_none() {
        push!(report, Error, "Failed to read crate at {}", new.display());
    }
    if let (Some(old), Some(new)) = (old_crate, new_crate) {
        compare_crates(&mut report, &old, &new);
    }

    report
}

fn fmt_path(path: &ast::Path) -> String {
    use std::fmt::Write;

    let mut res = String::new();
    let mut first = !path.global;
    for thing in &path.segments {
        if first {
            first = false;
        } else {
            res.push_str("::");
        }
        let _ = write!(res, "{}", thing.identifier.name);
    }
    res
}
