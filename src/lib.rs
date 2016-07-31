//! Some doc
#![feature(rustc_private)]
#![allow(unused_imports)]
extern crate syntax;

mod utils;

use std::rc::Rc;
use std::path::Path;

use syntax::ast::{self, StmtKind, Item, ItemKind, Attribute, MetaItemKind, Visibility, Crate};
use syntax::codemap::{self, Spanned};
use syntax::parse::parser::Parser;
use syntax::parse::{lexer, ParseSess};
use syntax::errors::Handler;
use syntax::errors::emitter::ColorConfig;

use utils::Indent;

fn parse_crate(file: &Path) -> Option<Crate> {
    let cm = Rc::new(codemap::CodeMap::new());
    let sh = Handler::with_tty_emitter(ColorConfig::Never, None, false, false, Some(cm.clone()));
    let ps = ParseSess::with_span_handler(sh, cm);
    let fm = ps.codemap().load_file(file).unwrap();
    let srdr = lexer::StringReader::new(&ps.span_diagnostic, fm);
    let mut p = Parser::new(&ps, Vec::new(), Box::new(srdr));
    // who knows why this is needed
    (|p: &mut Parser| p.parse_crate_mod().ok())(&mut p)
}

fn show_vec<T: ::std::fmt::Debug>(name: &str, vec: &[T]) {
    println!("{} ({}):", name, vec.len());
    for item in vec {
        println!("    {:?}", item);
    }
}

fn show_item(indent: Indent, item: &Item) {
    use syntax::ast::ItemKind::*;
    //show_attrs(indent, &item.attrs);

    print!("{}", indent);
    let vis = match item.vis {
        Visibility::Public => "pub ",
        Visibility::Crate(..) => "pub(crate) ",
        Visibility::Restricted { .. } => "pub(restricted) ",
        Visibility::Inherited => "",
    };

    match item.node {
        ExternCrate(Some(ref name)) => println!("{}{}extern crate {} as {};", indent, vis, name, item.ident),
        ExternCrate(None) => println!("{}{}extern crate {};", indent, vis, item.ident),
        Use(ref path) => {}
        _ => println!("{}{}unknown", indent, vis),
    }
}

pub fn main() {
    let cmod = parse_crate("src/lib.rs".as_ref()).unwrap();
    show_vec("attrs", &cmod.attrs);
    show_vec("config", &cmod.config);
    show_vec("macros", &cmod.exported_macros);
    println!("items ({}):", cmod.module.items.len());
    for thing in &cmod.module.items {
        show_item(Indent::new(), thing);
    }
}
