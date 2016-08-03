//! Some doc
#![feature(rustc_private)]
#![allow(dead_code)]
extern crate syntax;

#[macro_use]
pub mod report;
mod compare;
mod cfg;

use std::path::Path;

use syntax::ast::{self, Crate};

pub use compare::compare_crates;

pub fn parse_crate(file: &Path) -> Option<Crate> {
    use std::rc::Rc;
    use syntax::codemap;
    use syntax::parse::parser::Parser;
    use syntax::parse::{lexer, ParseSess};
    use syntax::errors::Handler;
    use syntax::errors::emitter::ColorConfig;

    let cm = Rc::new(codemap::CodeMap::new());
    let sh = Handler::with_tty_emitter(ColorConfig::Never, None, false, false, Some(cm.clone()));
    let ps = ParseSess::with_span_handler(sh, cm);
    let fm = ps.codemap().load_file(file).unwrap();
    let srdr = lexer::StringReader::new(&ps.span_diagnostic, fm);
    let mut p = Parser::new(&ps, Vec::new(), Box::new(srdr));
    // who knows why this is needed
    (|p: &mut Parser| p.parse_crate_mod().ok())(&mut p)
}

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

    report.strip_lazy();
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
