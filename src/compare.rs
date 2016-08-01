//! comparison engine
#![allow(unused_variables)]

use syntax::ast::*;

use report::*;
use cfg::cfg_subset;

fn todo(r: &mut Report, msg: &str) {
    push!(r, Debug, "TODO: {}", msg);
}

pub fn compare_crates(r: &mut Report, old: &Crate, new: &Crate) {
    if !cfg_subset(r, &old.attrs, &new.attrs) {
        push!(r, Breaking, "New crate reduces #[cfg] coverage");
    }
    compare_macros(r, &old.exported_macros, &new.exported_macros);
    compare_mods(r, &old.module, &new.module);
}

fn compare_macros(r: &mut Report, old: &[MacroDef], new: &[MacroDef]) {
    todo(r, "compare exported macros");
}

fn compare_mods(r: &mut Report, old: &Mod, new: &Mod) {
    use syntax::ast::ItemKind::*;
    for item in &old.items {
        if !is_public(item) { continue }

        match item.node {
            _ => push!(r, Debug, "Item \"{}\" has unhandled kind: {}", item.ident, item.node.descriptive_variant()),
        }
    }
}

fn is_public(item: &Item) -> bool {
    use syntax::ast::ItemKind::*;
    match item.node {
        // these nodes are always effectively public
        ForeignMod(..) | DefaultImpl(..) | Impl(..) | Mac(..) => true,
        // otherwise, check the visibility field
        _ => item.vis == Visibility::Public
    }
}
