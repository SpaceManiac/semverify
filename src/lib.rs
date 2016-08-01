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
        report.items.push(report::ReportItem {
            severity: report::Severity::Error,
            text: format!("Failed to read crate at {}", old.display())
        })
    }
    if new_crate.is_none() {
        report.items.push(report::ReportItem {
            severity: report::Severity::Error,
            text: format!("Failed to read crate at {}", new.display())
        })
    }
    if let (Some(old), Some(new)) = (old_crate, new_crate) {
        compare_crates(&mut report, &old, &new);
    }

    report
}

fn show_vec<T: ::std::fmt::Debug>(name: &str, vec: &[T]) {
    println!("{} ({}):", name, vec.len());
    for item in vec {
        println!("    {:?}", item);
    }
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

fn show_item(indent: Indent, item: &Item) {
    use syntax::ast::ItemKind::*;
    //show_attrs(indent, &item.attrs);


    match item.node {
        ExternCrate(Some(ref name)) => println!("extern crate {} as {};", name, item.ident),
        ExternCrate(None) => println!("extern crate {};", item.ident),
        Use(ref path) => match path.node {
            ast::ViewPath_::ViewPathSimple(ref ident, ref path) => {
                println!("use {} as {}", fmt_path(path), ident);
            }
            ast::ViewPath_::ViewPathGlob(ref path) => {
                println!("use {}::*", fmt_path(path))
            }
            ast::ViewPath_::ViewPathList(ref path, ref items) => {
                print!("use {}::{{", fmt_path(path));
                let mut first = true;
                for item in items {
                    if first {
                        first = false;
                    } else {
                        print!(", ");
                    }
                    match item.node {
                        ast::PathListItemKind::Ident { ref name, rename: Some(ref rename), .. } => {
                            print!("{} as {}", name, rename);
                        }
                        ast::PathListItemKind::Ident { ref name, rename: None, .. } => {
                            print!("{}", name);
                        }
                        ast::PathListItemKind::Mod { rename: Some(rename), .. } => {
                            print!("self as {}", rename);
                        }
                        ast::PathListItemKind::Mod { rename: None, .. } => {
                            print!("self");
                        }
                    }
                }
                println!("}}");
            }
        },
        Static(..) => println!("static {}", item.ident),
        Const(..) => println!("const {}", item.ident),
        Fn(..) => println!("fn {}", item.ident),
        Mod(ref module) => {
            println!("mod {}", item.ident);
            for item in &module.items {
                show_item(indent.next(), item);
            }
        }
        ForeignMod(ref module) => {
            println!("extern {}", module.abi);
        }
        Enum(..) => println!("enum {}", item.ident),
        Struct(..) => println!("struct {}", item.ident),
        Trait(..) => println!("trait {}", item.ident),
        Impl(..) => println!("impl {}", item.ident),
        Mac(..) => {},
        DefaultImpl(..) => {},
        Ty(..) => {},
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
