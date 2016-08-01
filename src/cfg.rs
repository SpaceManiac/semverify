use syntax::ast::*;
use report::*;

/// Check that targets covered by `old` are a subset of those by `new`.
pub fn cfg_subset(r: &mut Report, old: &[Attribute], new: &[Attribute]) -> bool {
    let new_cfg = match cfg_from_all_attrs(r, new) {
        Some(cfg) => cfg,
        None => return true, // new is unconditional
    };
    let old_cfg = match cfg_from_all_attrs(r, old) {
        Some(cfg) => cfg,
        None => return false, // old is unconditional, but new is not
    };
    push!(r, Note, "TODO: Compare #[cfg] trees with more detail");
    push!(r, Debug, "Old = {:?}", old_cfg);
    push!(r, Debug, "New = {:?}", new_cfg);
    true
}

/// None = unencumbered by configs
fn cfg_from_all_attrs(r: &mut Report, attrs: &[Attribute]) -> Option<Config> {
    let all: Vec<Config> = attrs.iter().flat_map(|a| cfg_from_attr(r, a)).collect();
    match all.len() {
        0 | 1 => all.into_iter().next(),
        _ => Some(Config::All(all)),
    }
}

fn cfg_from_attr(r: &mut Report, attr: &Attribute) -> Option<Config> {
    match attr.node.value.node {
        MetaItemKind::List(ref string, ref items) if &**string == "cfg" && items.len() == 1 => {
            Some(cfg_from_meta(r, &items[0].node))
        }
        _ => None
    }
}

fn cfg_from_meta(r: &mut Report, attr: &MetaItemKind) -> Config {
    use syntax::ast::MetaItemKind::*;
    match *attr {
        Word(ref string) => match &**string {
            "test" => Config::Test,
            "debug_assertions" => Config::DebugAssertions,
            "unix" | "windows" =>
                Config::TargetProperty("target_family".into(), LitKind::Str(string.clone(), StrStyle::Cooked)),
            _ => Config::Flag(string.to_string()),
        },
        List(ref string, ref items) => match &**string {
            "all" => Config::All(items.iter().map(|i| cfg_from_meta(r, &i.node)).collect()),
            "any" => Config::Any(items.iter().map(|i| cfg_from_meta(r, &i.node)).collect()),
            "not" => if items.len() == 1 {
                Config::Not(Box::new(cfg_from_meta(r, &items[0].node)))
            } else {
                push!(r, Error, "Non-unary #[cfg(not())]");
                Config::Flag("not".into())
            },
            _ => {
                push!(r, Error, "Unknown #[cfg] list: {}(...)", string);
                Config::Flag(string.to_string())
            }
        },
        NameValue(ref string, ref lit) => match &**string {
            "feature" => Config::Feature(lit.node.clone()),
            _ => {
                push!(r, Error, "Unknown #[cfg] key-value pair: {} = {:?}", string, lit.node);
                Config::Flag(string.to_string())
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum Config {
    Not(Box<Config>),
    All(Vec<Config>),
    Any(Vec<Config>),
    Test,
    DebugAssertions,
    TargetProperty(String, LitKind),
    Feature(LitKind),
    Flag(String),
}
