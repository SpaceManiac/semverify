//! Some doc
#![feature(rustc_private)]
#![allow(dead_code)]
extern crate syntax;
extern crate syntax_ext;
extern crate rustc;
extern crate rustc_resolve;
extern crate rustc_errors;

#[macro_use]
pub mod report;
mod compare;
mod cfg;

use std::path::Path;

use syntax::ast::{self, Crate};

pub use compare::compare_crates;

pub fn parse_crate(file: &Path) -> Option<Crate> {
    use std::rc::Rc;
    use syntax::codemap::CodeMap;
    use syntax::parse::parser::Parser;
    use syntax::parse::{lexer, ParseSess};
    use syntax::errors::Handler;
    use syntax::errors::emitter::ColorConfig;
    use rustc::session;
    use rustc_resolve::Resolver;

    // Set up the parser environment and read a crate
    let cm = Rc::new(CodeMap::new());
    let sh = Handler::with_tty_emitter(ColorConfig::Auto, None, false, false, Some(cm.clone()));
    let ps = ParseSess::with_span_handler(sh, cm);
    let fm = ps.codemap().load_file(file).unwrap();
    let reader = lexer::StringReader::new(&ps.span_diagnostic, fm);
    let mut p = Parser::new(&ps, Vec::new(), Box::new(reader));
    let krate = match p.parse_crate_mod() {
        Ok(c) => c,
        Err(_) => return None,
    };

    // I have been informed that things go wrong if you don't perform
    // configuration before macro expansion. In want of a real solution, this
    // advice is being blatantly ignored.

    // Expand syntax extensions ahead of time
    let ecfg = syntax::ext::expand::ExpansionConfig::default("__".to_string());
    let mut ml = syntax::ext::base::DummyMacroLoader; // TODO: replace with loading from other crates
    let mut ext_ctx = syntax::ext::base::ExtCtxt::new(&ps, krate.config.clone(), ecfg, &mut ml);
    // register built-in #[derive]s, so we can detect them as trait impls
    syntax_ext::deriving::register_all(&mut ext_ctx.syntax_env);
    // perform the expansion
    let (krate, macro_names) = syntax::ext::expand::expand_crate(ext_ctx, vec![], krate);

    // Assign node IDs: finishes expanding certain macro nodes
    let session = session::build_session(
        session::config::basic_options(),
        &rustc::dep_graph::DepGraph::new(false),
        Some(file.to_owned()),
        rustc_errors::registry::Registry::new(&[]),
        Rc::new(rustc::middle::cstore::DummyCrateStore),
    );
    let resolver_arenas = Resolver::arenas();
    let mut resolver = Resolver::new(&session, rustc_resolve::MakeGlobMap::No, &resolver_arenas);
    let krate = resolver.assign_node_ids(krate);

    Some(krate)
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
