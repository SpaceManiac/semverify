//! comparison engine
#![allow(unused_variables)]

use std::borrow::Borrow;

use syntax::ast::*;

use report::*;
use cfg;

macro_rules! find_item_l {
    ($r:expr; $kind:tt $var:ident: $orig:expr; in $iter:expr; $b:block) => {{
        let name = $orig.ident.name;
        let (mut any_found, mut pub_found, mut kind_found) = (false, false, false);
        for $var in $iter {
            if $var.ident.name == name {
                any_found = true;
                if is_public($var) {
                    pub_found = true;
                    if cfg::intersects($r, &$orig.attrs, &$var.attrs) && $b {
                        kind_found = true;
                    }
                }
            }
        }
        let kind = stringify!($kind);
        if !any_found {
            push!($r, Breaking, "{} {} was removed", kind, name);
        } else if !pub_found {
            push!($r, Breaking, "{} {} was made private", kind, name);
        } else if !kind_found {
            push!($r, Breaking, "{0} {1} is no longer a {0}", kind, name);
        }
    }}
}

fn todo(r: &mut Report, msg: &str) {
    push!(r, Debug, "TODO: {}", msg);
}

pub fn compare_crates(r: &mut Report, old: &Crate, new: &Crate) {
    if !cfg::subset(r, &old.attrs, &new.attrs) {
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

    let mut child_mods = Vec::new();

    // Look for back-compat breakages
    for item in &old.items {
        if !is_public(item) { continue }

        macro_rules! find_item {
            ($kind:tt $var:ident; $b:block) => {
                find_item_l!(r; $kind $var: item; in &new.items; $b)
            }
        }

        match item.node {
            // Child module: push to list for later consumption (improves tree structure)
            Mod(ref module) => find_item!(mod new_item; {
                if let Mod(ref new_module) = new_item.node {
                    child_mods.push((item.ident.name, module, new_module));
                    true
                } else {
                    false
                }
            }),
            // Consts and statics
            Const(ref ty, _) => find_item!(const new_item; {
                if let Const(ref new_ty, _) = new_item.node {
                    if !types_equal(r, ty, new_ty) {
                        changed!(r, Breaking, "const {}'s type", (ty => new_ty), item.ident);
                    }
                    true
                } else if let Static(ref new_ty, mutability, _) = new_item.node {
                    if mutability != Mutability::Immutable {
                        push!(r, Breaking, "const {} replaced by static mut", item.ident);
                    } else {
                        push!(r, Minor, "const {} replaced by static", item.ident);
                    }
                    if !types_equal(r, ty, new_ty) {
                        changed!(r, Breaking, "const {}'s type", (ty => new_ty), item.ident);
                    }
                    true
                } else {
                    false
                }
            }),
            Static(ref ty, mutability, _) => find_item!(static new_item; {
                if let Const(ref new_ty, _) = new_item.node {
                    push!(r, Breaking, "static {} replaced by const", item.ident);
                    if !types_equal(r, ty, new_ty) {
                        changed!(r, Breaking, "static {}'s type", (ty => new_ty), item.ident);
                    }
                    true
                } else if let Static(ref new_ty, new_mutability, _) = new_item.node {
                    if !types_equal(r, ty, new_ty) {
                        changed!(r, Breaking, "static {}'s type", (ty => new_ty), item.ident);
                    }
                    if mutability != new_mutability {
                        changed!(r, Breaking, "static {}'s mutability", (mutability => new_mutability), item.ident);
                    }
                    true
                } else {
                    false
                }
            }),
            // Unhandled types
            _ => push!(r, Note, "Item \"{}\" has unhandled kind: {}", item.ident, item.node.descriptive_variant()),
        }
    }

    // Look for additions

    // Recurse to child modules
    for (name, old_child, new_child) in child_mods {
        compare_mods(r, old_child, new_child);
    }
}

fn types_equal(r: &mut Report, lhs: &Ty, rhs: &Ty) -> bool {
    // TODO: a more robust comparison not based on pretty-printing
    format!("{:?}", lhs) == format!("{:?}", rhs)
}

fn use_defines_name(r: &mut Report, vp: &ViewPath, name: Name) -> bool {
    match vp.node {
        ViewPath_::ViewPathSimple(ref ident, _) => ident.name == name,
        ViewPath_::ViewPathGlob(_) => {
            push!(r, Warning, "Glob imports are not yet handled");
            false
        }
        ViewPath_::ViewPathList(ref path, ref items) => items.iter().any(|item| match item.node {
            PathListItemKind::Ident { rename: Some(ref ident), .. } |
            PathListItemKind::Ident { name: ref ident, .. } |
            PathListItemKind::Mod { rename: Some(ref ident), .. } =>
                ident.name == name,
            PathListItemKind::Mod { .. } => {
                path.segments.last().map(|p| p.identifier.name == name).unwrap_or(false)
            }
        })
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
