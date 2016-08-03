//! Reporting data structures

use std::borrow::Cow;

/// Ordered severity levels for report items.
///
/// The severity of a report item is determined by RFC 1105.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Severity {
    /// Tool debug
    Debug,
    /// Tool note
    Note,
    /// Change requiring bumping of minor release number
    Minor,
    /// Tool warning
    Warning,
    /// Breaking change that is not considered to require a major release
    Breaking,
    /// Change requiring bumping of major release number
    Major,
    /// Tool error
    Error,
}

/// Strictness level of a report item.
///
/// `Lazy` report items are only included in the final report if they have
/// `Strict` children.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Strictness {
    /// Deleted if no recursive child is Strict.
    Lazy,
    /// Deleted if parent is deleted.
    Inherit,
    /// Never deleted.
    Strict,
}

/// An informational entry in the report.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ReportItem {
    pub strict: Strictness,
    pub severity: Severity,
    pub text: Cow<'static, str>,
}

/// A node in the tree structure of the report.
#[derive(Debug)]
pub struct Report {
    pub item: ReportItem,
    pub children: Vec<Report>,
}

impl Report {
    /// Construct a new, empty report.
    pub fn new() -> Report {
        Report::from(ReportItem {
            strict: Strictness::Strict,
            severity: Severity::Note,
            text: "Crate root".into(),
        })
    }

    /// Calculate the highest severity item contained in this report.
    pub fn highest_severity(&self) -> Severity {
        self.children.iter().map(Report::highest_severity).max().unwrap_or(self.item.severity)
    }

    /// Strip lazy entries. Returns whether this entry might be deleted.
    pub fn strip_lazy(&mut self) -> bool {
        // delete all Lazy children
        delete_if(&mut self.children, Report::strip_lazy);
        // delete us if we are Lazy and no children are Strict
        self.item.strict == Strictness::Lazy &&
            self.children.iter().all(|c| c.item.strict < Strictness::Strict)
    }

    /// Insert a new item into this report, returning it.
    pub fn push(&mut self, child: ReportItem) -> &mut Report {
        self.children.push(child.into());
        self.children.last_mut().unwrap()
    }
}

impl From<ReportItem> for Report {
    fn from(val: ReportItem) -> Report {
        Report { item: val, children: Vec::new() }
    }
}

macro_rules! push {
    (@_format $string:expr) => ($string.into());
    (@_format $string:expr, $($rest:tt)*) => (format!($string, $($rest)*).into());
    ($report:expr, $severity:ident, $($rest:tt)*) => {
        $report.push(::report::ReportItem {
            strict: ::report::Strictness::Strict,
            severity: ::report::Severity::$severity,
            text: push!(@_format $($rest)*),
        })
    };
    ($report:expr, $strict:ident $severity:ident, $($rest:tt)*) => {
        $report.push(::report::ReportItem {
            strict: ::report::Strictness::$strict,
            severity: ::report::Severity::$severity,
            text: push!(@_format $($rest)*),
        })
    };
}

macro_rules! changed {
    ($report:expr, $severity:ident, $what:expr, ($was:expr => $now:expr) $($rest:tt)*) => {
        push!($report, $severity,
            concat!($what, " has changed:\n  Was: {:?}\n  Now: {:?}")
            $($rest)*, $was, $now
        )
    }
}

fn delete_if<T, F>(vec: &mut Vec<T>, mut f: F)
    where F: FnMut(&mut T) -> bool
{
    let len = vec.len();
    let mut del = 0;
    {
        let v = &mut **vec;

        for i in 0..len {
            if f(&mut v[i]) {
                del += 1;
            } else if del > 0 {
                v.swap(i - del, i);
            }
        }
    }
    if del > 0 {
        vec.truncate(len - del);
    }
}
