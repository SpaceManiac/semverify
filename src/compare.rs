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
// The RFC is not exhaustive, and serves primarily as a guide for the severity of
// changes which are not obviously Major.
//
// Note that behavioral changes that are not reflected in public signatures are
// not covered by the RFC, and probably cannot be checked by a tool like this.
// Instead, they should be caught using a comprehensive test suite.

use syntax::ast::*;
use syntax::abi::Abi;

use report::*;
use cfg::Config;

fn todo(r: &mut Report, msg: &str) {
    push!(r, Debug, "TODO: {}", msg);
}

/// Generate a report on changes described in the "Crates" section.
pub fn compare_crates(r: &mut Report, old: &Crate, new: &Crate) {
    let old_config = Config::new(r, &old.attrs);
    let new_config = Config::new(r, &new.attrs);
    if !old_config.subset(&new_config) {
        push!(r, Major, "New crate reduces #[cfg] coverage\n  Was: {}\n  Now: {}", old_config, new_config);
    } else if !new_config.subset(&old_config) {
        push!(r, Minor, "New crate increases #[cfg] coverage\n  Was: {}\n  Now: {}", old_config, new_config);
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

    // TODO: include more detailed #[cfg] printouts in the messages

    // Major: "renaming/moving/removing any public items."
    // Check that every item is found at the same path in the new version, and
    // furthermore perform checks that the new item agrees with the old item
    // in an item-specific way.
    for item in &old.items {
        if !is_public(item) { continue }

        macro_rules! find_item {
            ($kind_name:expr; $closure:expr) => {{
                let kind = $kind_name;
                let r = push!(r, Note, "{} {}", kind, item.ident.name);
                let result = search_items(r, item, new.items.iter().map(|x| &**x), $closure);
                if !result.found_name {
                    push!(r, Major, "removed");
                } else if !result.found_pub {
                    push!(r, Major, "made private");
                } else if !result.item_cfg.subset(&result.found_cfgs) {
                    push!(r, Major, "availability narrowed:\n  Was: {}\n  Now: {}", result.item_cfg, result.found_cfgs);
                }
            }}
        }

        match item.node {
            // Child module: push to list for later consumption (improves tree structure)
            Mod(ref module) => find_item!("mod"; |r, new| if let Mod(ref new_module) = new.node {
                child_mods.push((item, module, new, new_module));
                true
            } else { false }),

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
            // Breaking: "adding any inherent items."

            // TODO: Type aliases
            // See: Signatures in type definitions

            // Consts and statics
            Const(ref ty, _) => find_item!("const"; |r, new| match new.node {
                Const(ref new_ty, _) => {
                    if !types_equal(r, ty, new_ty) {
                        changed!(r, Major, "const {}'s type", (ty => new_ty), item.ident);
                    }
                    true
                }
                Static(ref new_ty, mutability, _) => {
                    if mutability != Mutability::Immutable {
                        push!(r, Major, "const {} replaced by static mut", item.ident);
                    } else {
                        push!(r, Minor, "const {} replaced by static", item.ident);
                    }
                    if !types_equal(r, ty, new_ty) {
                        changed!(r, Major, "const {}'s type", (ty => new_ty), item.ident);
                    }
                    true
                }
                _ => false
            }),
            Static(ref ty, mutability, _) => find_item!("static"; |r, new| match new.node {
                Const(ref new_ty, _) => {
                    push!(r, Major, "static {} replaced by const", item.ident);
                    if !types_equal(r, ty, new_ty) {
                        changed!(r, Major, "static {}'s type", (ty => new_ty), item.ident);
                    }
                    true
                }
                Static(ref new_ty, new_mutability, _) => {
                    if !types_equal(r, ty, new_ty) {
                        changed!(r, Major, "static {}'s type", (ty => new_ty), item.ident);
                    }
                    if mutability != new_mutability {
                        changed!(r, Major, "static {}'s mutability", (mutability => new_mutability), item.ident);
                    }
                    true
                }
                _ => false
            }),
            // Functions
            Fn(ref decl, unsafety, constness, abi, ref generics, _) => {
                if decl.variadic {
                    push!(r, Error, "non-foreign fn {} is variadic", item.ident);
                }
                find_item!("fn"; |r, new| if let Fn(ref new_decl, new_unsafety, new_constness, new_abi, ref new_generics, _) = new.node {
                    let r = push!(r, Note, "fn {}", item.ident.name);
                    if constness == Constness::Const && new_constness == Constness::NotConst {
                        push!(r, Major, "const qualifier removed");
                    }
                    compare_functions(r,
                        (decl, generics, unsafety, abi),
                        (new_decl, new_generics, new_unsafety, new_abi));
                    true
                } else { false })
            }
            // Unhandled types
            _ => { push!(r, Warning, "Unhandled: {} {}", item.node.descriptive_variant(), item.ident); },
        }
    }

    // "Minor change: adding new public items."
    // All the heavy lifting of checking an old version vs. a new version is
    // done above. Here are checks for new public items which did not exist
    // before. Due to glob imports, these are technically Breaking, but it
    // would impact report clarity to report this accurately. Instead, they
    // are reported as Minor.
    for item in &new.items {
        if !is_public(item) { continue }

        macro_rules! find_item {
            ($kind_name:expr; $closure:expr) => {{
                let kind = $kind_name;
                let r = push!(r, Lazy Note, "{} {}", kind, item.ident.name);
                let result = search_items(r, item, old.items.iter().map(|x| &**x), $closure);
                if !result.found_name {
                    push!(r, Minor, "added");
                } else if !result.found_pub {
                    push!(r, Minor, "made public");
                } else if !result.item_cfg.subset(&result.found_cfgs) {
                    push!(r, Minor, "availability widened:\n  Was: {}\n  Now: {}", result.found_cfgs, result.item_cfg);
                }
            }}
        }

        match item.node {
            // Child modules
            Mod(ref module) => find_item!("mod"; |_, old| match old.node {
                Mod(_) => true, _ => false,
            }),
            // Consts and statics
            Const(ref ty, _) => find_item!("const"; |_, old| {
                match old.node { Const(..) | Static(..) => true, _ => false }
            }),
            Static(ref ty, mutability, _) => find_item!("static"; |_, old| {
                match old.node { Const(..) | Static(..) => true, _ => false }
            }),
            // Functions
            Fn(ref decl, unsafety, constness, abi, ref generics, _) => find_item!("fn"; |_, old| {
                match old.node { Fn(..) => true, _ => false }
            }),
            _ => {}
        }
    }

    // Recurse to child modules
    for (item, module, new_item, new_module) in child_mods {
        let r = push!(r, Note, "mod {}", item.ident.name);
        let old_config = Config::new(r, &item.attrs);
        old_config.report(r, &Config::True, "");
        let r = Config::new(r, &new_item.attrs).report(r, &old_config, "Comparing with ");
        compare_mods(r, module, new_module);
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
fn compare_type_generics(r: &mut Report, old: &Generics, new: &Generics) {
    // TODO
}

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
fn compare_functions(r: &mut Report,
    (decl, generics, unsafety, abi): (&FnDecl, &Generics, Unsafety, Abi),
    (new_decl, new_generics, new_unsafety, new_abi): (&FnDecl, &Generics, Unsafety, Abi))
{
    if unsafety != new_unsafety {
        changed!(r, Major, "unsafety", (unsafety => new_unsafety));
    }
    if abi != new_abi {
        changed!(r, Major, "abi", (abi => new_abi));
    }
    // TODO
}

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

struct SearchResult {
    /// The #[cfg] flags on the input item
    item_cfg: Config,
    /// The union of #[cfg] flags on matching items
    found_cfgs: Config,
    /// Found an item with a matching name
    found_name: bool,
    /// ... that is also public
    found_pub: bool,
    /// ... that was also a matching kind
    found_kind: bool,
}

fn search_items<'a, I, F>(r: &mut Report, orig: &Item, iter: I, mut f: F) -> SearchResult where
    F: FnMut(&mut Report, &'a Item) -> bool,
    I: IntoIterator<Item=&'a Item>,
{
    let mut result = SearchResult {
        item_cfg: Config::new(r, &orig.attrs),
        found_cfgs: Config::False,
        found_name: false,
        found_pub: false,
        found_kind: false,
    };
    result.item_cfg.report(r, &Config::True, "");

    for item in iter {
        let item: &Item = &item;
        if item.ident.name == orig.ident.name {
            result.found_name = true;
            if is_public(item) {
                result.found_pub = true;
                let local_cfg = Config::new(r, &item.attrs);
                if result.item_cfg.intersects(&local_cfg) {
                    let r = local_cfg.report(r, &result.item_cfg, "Comparing with ");
                    if f(r, item) {
                        result.found_cfgs.union(local_cfg);
                        result.found_kind = true;
                    }
                }
            }
        }
    }

    result.found_cfgs.simplify();
    result
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
