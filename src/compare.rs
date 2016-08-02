//! comparison engine
#![allow(unused_variables)]

// The definite reference for what changes are what severity is here:
// https://github.com/rust-lang/rfcs/blob/master/text/1105-api-evolution.md
//
// The RFC describes and justifies three tiers of severity:
// - Minor: never breaks downstream crates, only requires a minor version bump
// - Breaking: might break downstream crates, but still only requires a minor version bump
// - Major: might break downstream crates, requirs a major version bump
//
// The individual subheadings in the RFC are marked Minor or Major, with a note for
// Minor changes which are Breaking. These subheadings are explicitly called out
// adjacent to the code which checks for them, with additional notes if needed.
//
// Note that behavioral changes that are not reflected in public signatures are
// not covered by the RFC, and probably cannot be checked by a tool like this.
// Instead, they should be caught using a comprehensive test suite.

use syntax::ast::*;

use report::*;
use cfg;

macro_rules! find_item_l {
    ($r:expr; $kind:tt $var:ident: $orig:expr; in $iter:expr; $b:block) => {{
        let name = $orig.ident.name;
        let (mut any_found, mut pub_found, mut kind_found) = (false, false, false);

        let our_config = cfg::Config::new($r, &$orig.attrs);
        let mut their_config = cfg::Config::False;

        for $var in $iter {
            if $var.ident.name == name {
                any_found = true;
                if is_public($var) {
                    pub_found = true;
                    let current_config = cfg::Config::new($r, &$var.attrs);
                    if our_config.intersects(&current_config) && $b {
                        their_config.union(current_config);
                        kind_found = true;
                    }
                }
            }
        }
        let kind = stringify!($kind);
        if !any_found {
            push!($r, Major, "{} {} was removed", kind, name);
        } else if !pub_found {
            push!($r, Major, "{} {} was made private", kind, name);
        } else if !kind_found {
            push!($r, Major, "{0} {1} is no longer a {0}", kind, name);
        } else if !our_config.subset(&their_config) {
            their_config.simplify(); // TODO: maybe move this up sometime
            push!($r, Major, "{} {} has been narrowed:\n  Was: {:?}\n  Now: {:?}", kind, name, our_config, their_config);
        }
    }}
}

fn todo(r: &mut Report, msg: &str) {
    push!(r, Debug, "TODO: {}", msg);
}

/// Generate a report on changes described in the "Crates" section.
pub fn compare_crates(r: &mut Report, old: &Crate, new: &Crate) {
    if !cfg::subset(r, &old.attrs, &new.attrs) {
        push!(r, Major, "New crate reduces #[cfg] coverage");
    }
    // TODO: Major: "going from stable to nightly"
    // - if `new` has #[feature(...)] but `old` does not
    // TODO: Minor: "altering the use of Cargo features"
    compare_macros(r, &old.exported_macros, &new.exported_macros);
    compare_mods(r, &old.module, &new.module);
}

fn compare_macros(r: &mut Report, old: &[MacroDef], new: &[MacroDef]) {
    // TODO: compare exported macros
}

fn compare_mods(r: &mut Report, old: &Mod, new: &Mod) {
    use syntax::ast::ItemKind::*;
    macro_rules! debug {
        ($($rest:tt)*) => { push!(r, Debug, $($rest)*) }
    }

    let mut child_mods = Vec::new();

    // Major: "renaming/moving/removing any public items."
    // Check that every item is found at the same path in the new version, and
    // furthermore perform checks that the new item agrees with the old item
    // in an item-specific way.
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

            // TODO: Structs
            // See: Signatures in type definitions
            // Major: "adding a private field when all current fields are public."
            // Major: "adding a public field when no private field exists."
            // Minor: "adding or removing private fields when at least one already exists (before and after the change)."
            // - "For tuple structs, this is only a minor change if furthermore all fields are currently private."
            // Breaking: "going from a tuple struct with all private fields (with at least one field) to a normal struct, or vice versa."

            // TODO: Enums
            // See: Signatures in type definitions
            // Major: "adding new variants."
            // - Merely Breaking if enum has indicated nonexaustiveness in a nonstandard way
            // Major: "adding new fields to a variant."

            // TODO: Traits
            // Major: "adding a non-defaulted item."
            // Major: "any non-trivial change to item signatures."
            // Breaking: "adding a defaulted item."
            // Minor: "adding a defaulted type parameter."

            // TODO: Trait implementations
            // Major: "implementing any "fundamental" trait."
            // Breaking: "implementing any non-fundamental trait."

            // TODO: Inherent implementations
            // Brekaing: "adding any inherent items."

            // TODO: Type aliases
            // See: Signatures in type definitions

            // Consts and statics
            Const(ref ty, _) => find_item!(const new_item; {
                if let Const(ref new_ty, _) = new_item.node {
                    if !types_equal(r, ty, new_ty) {
                        changed!(r, Major, "const {}'s type", (ty => new_ty), item.ident);
                    }
                    true
                } else if let Static(ref new_ty, mutability, _) = new_item.node {
                    if mutability != Mutability::Immutable {
                        push!(r, Major, "const {} replaced by static mut", item.ident);
                    } else {
                        push!(r, Minor, "const {} replaced by static", item.ident);
                    }
                    if !types_equal(r, ty, new_ty) {
                        changed!(r, Major, "const {}'s type", (ty => new_ty), item.ident);
                    }
                    true
                } else {
                    false
                }
            }),
            Static(ref ty, mutability, _) => find_item!(static new_item; {
                if let Const(ref new_ty, _) = new_item.node {
                    push!(r, Major, "static {} replaced by const", item.ident);
                    if !types_equal(r, ty, new_ty) {
                        changed!(r, Major, "static {}'s type", (ty => new_ty), item.ident);
                    }
                    true
                } else if let Static(ref new_ty, new_mutability, _) = new_item.node {
                    if !types_equal(r, ty, new_ty) {
                        changed!(r, Major, "static {}'s type", (ty => new_ty), item.ident);
                    }
                    if mutability != new_mutability {
                        changed!(r, Major, "static {}'s mutability", (mutability => new_mutability), item.ident);
                    }
                    true
                } else {
                    false
                }
            }),
            // Functions
            Fn(ref decl, unsafety, constness, abi, ref generics, _) => {
                if decl.variadic {
                    push!(r, Error, "non-foreign fn {} is variadic", item.ident);
                }
                debug!("decl = {:?}", decl);
                debug!("generics = {:?}", generics);
                debug!("{:?} {:?} {:?}", unsafety, constness, abi);
                find_item!(fn new_item; {
                    if let Fn(ref new_decl, new_unsafety, new_constness, new_abi, ref new_generics, _) = new_item.node {
                        if unsafety != new_unsafety {
                            changed!(r, Major, "fn {}'s unsafety", (unsafety => new_unsafety), item.ident);
                        }
                        if constness == Constness::Const && new_constness == Constness::NotConst {
                            push!(r, Major, "fn {} was made non-const", item.ident);
                        }
                        if abi != new_abi {
                            changed!(r, Major, "fn {}'s abi", (abi => new_abi), item.ident);
                        }
                        // TODO: actual signature comparison
                        true
                    } else {
                        false
                    }
                })
            }
            // Unhandled types
            _ => push!(r, Note, "Item \"{}\" has unhandled kind: {}", item.ident, item.node.descriptive_variant()),
        }
    }

    // Look for additions

    // Recurse to child modules
    for (name, old_child, new_child) in child_mods {
        compare_mods(r.nest(ReportItem::new(Severity::Note, format!("Inside mod {}", name))), old_child, new_child);
    }
}

// TODO: Signatures in type definitions
// Major: "tightening bounds."
// Minor: "loosening bounds."
// Minor: "adding defaulted type parameters."
// Minor: "generalizing to generics."
// - Minor: Foo(pub u8) to Foo<T = u8>(pub T)
// - Major: Foo<T = u8>(pub T, pub u8) to Foo<T = u8>(pub T, pub T)
// - Minor: Foo<T>(pub T, pub T) to Foo<T, U = T>(pub T, pub U)
fn types_equal(r: &mut Report, lhs: &Ty, rhs: &Ty) -> bool {
    // TODO: a more robust comparison not based on pretty-printing
    format!("{:?}", lhs) == format!("{:?}", rhs)
}

// TODO: Signatures in functions
// Note: In trait definitons, ALL the following changes are Major.
// Major: "adding/removing arguments."
// Breaking: "introducing a new type parameter."
// Minor: "generalizing to generics."
// - Minor: if the original type satisfies the new generic.
// - Major: if the generic has a bound not satisfied by the original type.
// - Can in principle cause type inference failures, but left Minor.
// - Minor: foo(_: &Trait) to foo<T: Trait + ?Sized>(t: &T)

// TODO: Lints
// Minor: "introducing new lint warnings/errors"
// - Any change which will cause downstream code to emit new lints
// - Can this even be checked for?

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
