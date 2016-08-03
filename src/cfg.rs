use std::collections::BTreeSet;
use std::fmt;
use syntax::ast::*;
use report::*;

/// A `#[cfg]` attribute represented as a boolean expression tree.
#[derive(PartialEq, Eq)]
pub enum Config {
    // combinators
    Not(Box<Config>),
    All(Vec<Config>),
    Any(Vec<Config>),
    // atoms
    TargetProperty(String, String),
    Feature(String),
    Flag(String),
    // internal use only
    True,
    False,
}

impl Config {
    /// Extract a Config from the given list of attributes.
    pub fn new(r: &mut Report, attrs: &[Attribute]) -> Config {
        cfg_from_attr_list(r, attrs).unwrap_or(Config::True)
    }

    /// Determine if this Config is a subset of another.
    ///
    /// Returns `true` if in all cases where this Config applies, so too
    /// does `other`.
    pub fn subset(&self, other: &Config) -> bool {
        if other.is_universal() {
            true
        // no short-circuit if self.is_universal(): other may be like Any([True, False])
        } else {
            !any(self, other, |vars| self.evaluate(vars) && !other.evaluate(vars))
        }
    }

    /// Determine if this Config overlaps with another.
    ///
    /// Returns `true` if in any case where this Config applies, so too
    /// does `other`.
    pub fn intersects(&self, other: &Config) -> bool {
        if other.is_universal() || self.is_universal() {
            true
        } else {
            any(self, other, |vars| self.evaluate(vars) && other.evaluate(vars))
        }
    }

    /// Determine if this Config is exactly equivalent to another.
    pub fn equivalent(&self, other: &Config) -> bool {
        if self.is_universal() && other.is_universal() {
            true
        } else {
            !any(self, other, |vars| self.evaluate(vars) ^ other.evaluate(vars))
        }
    }

    /// Make an entry in the report for this Config if needed.
    ///
    /// An entry is only made if this config is conditional with respect to
    /// the provided parent. `Config::True` can be used if needed.
    pub fn report<'a>(&self, r: &'a mut Report, parent: &Config, prefix: &str) -> &'a mut Report {
        if parent.subset(self) {
            r // also applies when self.is_universal()
        } else {
            push!(r, Inherit Note, "{}#[cfg({:?})]", prefix, self)
        }
    }

    /// Compute the union of this config and another.
    pub fn union(&mut self, mut other: Config) {
        if let Config::Any(ref mut inner) = *self {
            if let Config::Any(inner_2) = other {
                inner.extend(inner_2);
            } else {
                inner.push(other);
            }
            return
        }
        ::std::mem::swap(self, &mut other);
        if let Config::Any(ref mut inner) = *self {
            inner.push(other);
            return;
        }
        *self = Config::Any(vec![
            ::std::mem::replace(self, Config::Any(Vec::new())), // dummy value
            other
        ]);
    }

    /// Simplify this config, removing certain kinds of redundancies.
    pub fn simplify(&mut self) {
        let new_value;
        match *self {
            Config::Not(ref mut inner) => {
                inner.simplify();
                if **inner == Config::True {
                    new_value = Config::False;
                } else if **inner == Config::False {
                    new_value = Config::True;
                } else {
                    return
                }
            }
            Config::All(ref mut inner) => {
                for each in inner.iter_mut() {
                    each.simplify();
                }
                inner.retain(|i| *i != Config::True);
                if inner.len() == 0 {
                    new_value = Config::True;
                } else if inner.len() == 1 {
                    new_value = inner.remove(0);
                } else {
                    return
                }
            }
            Config::Any(ref mut inner) => {
                for each in inner.iter_mut() {
                    each.simplify();
                }
                inner.retain(|i| *i != Config::False);
                if inner.len() == 0 {
                    new_value = Config::False;
                } else if inner.len() == 1 {
                    new_value = inner.remove(0);
                } else {
                    return
                }
            }
            _ => return,
        }
        *self = new_value;
    }

    #[inline]
    fn is_universal(&self) -> bool {
        *self == Config::True
    }

    fn find_free_vars<'a>(&'a self, out: &mut BTreeSet<FreeVar<'a>>) {
        match *self {
            Config::Not(ref inner) => { inner.find_free_vars(out); }
            Config::All(ref inner) | Config::Any(ref inner) => {
                for i in inner { i.find_free_vars(out) }
            }
            Config::TargetProperty(ref key, ref val) => {
                out.insert(FreeVar::TargetProperty(key, val));
            }
            Config::Feature(ref name) => { out.insert(FreeVar::Feature(name)); }
            Config::Flag(ref name) => { out.insert(FreeVar::Flag(name)); }
            Config::True | Config::False => {},
        }
    }

    fn evaluate(&self, vars: &BTreeSet<&FreeVar>) -> bool {
        match *self {
            Config::Not(ref inner) => !inner.evaluate(vars),
            Config::All(ref inner) => inner.iter().all(|i| i.evaluate(vars)),
            Config::Any(ref inner) => inner.iter().any(|i| i.evaluate(vars)),
            Config::TargetProperty(ref key, ref val) => vars.contains(&&FreeVar::TargetProperty(key, val)),
            Config::Feature(ref name) => vars.contains(&&FreeVar::Feature(name)),
            Config::Flag(ref name) => vars.contains(&&FreeVar::Flag(name)),
            Config::True => true,
            Config::False => false,
        }
    }
}

/// Show this `Config` in a human-readable format, wrapping in #[cfg] if needed.
impl fmt::Display for Config {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Config::True => fmt.write_str("always available"),
            Config::False => fmt.write_str("never available"),
            _ => write!(fmt, "#[cfg({:?})]", self),
        }
    }
}

impl fmt::Debug for Config {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Config::Not(ref inner) => write!(fmt, "not({:?})", inner),
            Config::All(ref inner) => fmt_list(fmt, inner, "all("),
            Config::Any(ref inner) => fmt_list(fmt, inner, "any("),
            Config::TargetProperty(ref key, ref val) => write!(fmt, "{}={:?}", key, val),
            Config::Feature(ref name) => write!(fmt, "feature={:?}", name),
            Config::Flag(ref name) => write!(fmt, "{}", name),
            Config::True => fmt.write_str("__true"),
            Config::False => fmt.write_str("__false"),
        }
    }
}

fn fmt_list(fmt: &mut fmt::Formatter, inner: &[Config], start: &str) -> fmt::Result {
    use std::fmt::Write;

    try!(fmt.write_str(start));
    let mut first = true;
    for each in inner {
        if first {
            first = false;
        } else {
            try!(fmt.write_str(", "));
        }
        try!(write!(fmt, "{:?}", each));
    }
    fmt.write_char(')')
}

/// None = universal
fn cfg_from_attr_list(r: &mut Report, attrs: &[Attribute]) -> Option<Config> {
    let all: Vec<Config> = attrs.iter().flat_map(|attr| match attr.node.value.node {
        MetaItemKind::List(ref string, ref items) if &**string == "cfg" && items.len() == 1 => {
            if items.len() == 1 {
                Some(cfg_from_meta(r, &items[0].node))
            } else {
                push!(r, Error, "Non-unary #[cfg]: {:?}", items);
                None
            }
        }
        _ => None
    }).collect();
    match all.len() {
        0 | 1 => all.into_iter().next(),
        _ => Some(Config::All(all)),
    }
}

fn cfg_from_meta(r: &mut Report, attr: &MetaItemKind) -> Config {
    use syntax::ast::MetaItemKind::*;
    match *attr {
        Word(ref string) => match &**string {
            // NOTE: The default target_family values are ONLY "unix" and
            // "windows". Other values are not handled here, but writing out
            // `#[cfg(target_family="nonstandard_value")]` will work fine.
            "unix" | "windows" =>
                Config::TargetProperty("target_family".into(), string.to_string()),
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
        NameValue(ref string, ref lit) => {
            let string_lit = if let LitKind::Str(ref str_lit, _) = lit.node {
                str_lit.to_string()
            } else {
                push!(r, Error, "Non-string #[cfg]: {} = {:?}", string, lit);
                return Config::Flag(string.to_string())
            };
            match &**string {
                "feature" => Config::Feature(string_lit),
                "target_arch" | "target_os" | "target_family" |
                "target_env" | "target_endian" | "target_pointer_width" |
                "target_vendor" => {
                    Config::TargetProperty(string.to_string(), string_lit)
                }
                "target_has_atomic" => {
                    // may be true for multiple values; treat like a flag
                    Config::Flag(format!("target_has_atomic={:?}", string_lit))
                }
                _ => {
                    push!(r, Error, "Unknown #[cfg] key-value pair: {} = {:?}", string, lit.node);
                    Config::Flag(string.to_string())
                }
            }
        }
    }
}

fn any<F: Fn(&BTreeSet<&FreeVar>) -> bool>(one: &Config, other: &Config, f: F) -> bool {
    // compute free var set
    let mut free_vars = BTreeSet::new();
    one.find_free_vars(&mut free_vars);
    other.find_free_vars(&mut free_vars);

    // turn free var set into series of options to iterate
    let mut options: Vec<Vec<&FreeVar>> = vec![];
    for var in &free_vars {
        match *var {
            FreeVar::TargetProperty(ref key, _) => {
                match options.iter().position(|v| match *v[0] {
                    FreeVar::TargetProperty(ref key2, _) if key == key2 => true,
                    _ => false,
                }) {
                    Some(idx) => options[idx].push(var),
                    None => options.push(vec![var]),
                }
            }
            _ => options.push(vec![var]),
        }
    }

    // iterate over each possibility
    let mut positions = vec![0; options.len()];
    let mut set = BTreeSet::new();
    'outer: loop {
        // evaluate the current set of positions
        set.clear();
        for (&pos, options) in positions.iter().zip(&options) {
            if pos > 0 {
                set.insert(options[pos - 1]);
            }
        }
        if f(&set) {
            return true
        }

        // step to next set of positions, or break
        for (pos, options) in positions.iter_mut().zip(&options) {
            *pos += 1;
            if *pos > options.len() {
                *pos = 0;
                continue;
            }
            continue 'outer;
        }
        break;
    }

    false
}

#[derive(PartialEq, Eq, PartialOrd, Ord)]
enum FreeVar<'a> {
    TargetProperty(&'a str, &'a str),
    Feature(&'a str),
    Flag(&'a str),
}

#[test]
fn oh_boy_here_we_go() {
    // #[cfg(arbitrary)] and #[cfg(unix)] might intersect!
    // without knowing the meaning of "arbitrary", it can't be said for sure
    assert!(Config::Flag("arbitrary".into()).intersects(
        &Config::TargetProperty("target_family".into(), "unix".into())));

    assert!(!Config::TargetProperty("target_os".into(), "linux".into()).intersects(
        &Config::TargetProperty("target_os".into(), "windows".into())
    ));
    assert!(Config::Feature("one".into()).subset(&Config::Feature("one".into())));
    assert!(!Config::Feature("two".into()).subset(&Config::Feature("one".into())));
}
