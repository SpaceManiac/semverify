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

/// An informational entry in the report.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ReportItem {
    pub lazy: bool,
    pub severity: Severity,
    pub text: Cow<'static, str>,
}

impl ReportItem {
    pub fn new<S: Into<Cow<'static, str>>>(severity: Severity, message: S) -> ReportItem {
        ReportItem { lazy: false, severity: severity, text: message.into() }
    }

    pub fn lazy<S: Into<Cow<'static, str>>>(severity: Severity, message: S) -> ReportItem {
        ReportItem { lazy: true, severity: severity, text: message.into() }
    }
}

/// A node in the tree structure of the report.
#[derive(Debug)]
pub struct Report {
    pub item: ReportItem,
    pub children: Vec<Report>,
}

impl Report {
    pub fn new() -> Report {
        Report::from(ReportItem::new(Severity::Note, "Crate root"))
    }

    pub fn highest_severity(&self) -> Severity {
        self.children.iter().map(Report::highest_severity).max().unwrap_or(self.item.severity)
    }

    /// Strip lazy entries. Returns `false` if all entries were lazy.
    pub fn strip_lazy(&mut self) -> bool {
        retain_mut(&mut self.children, Report::strip_lazy);
        !self.item.lazy || !self.children.is_empty()
    }

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
    ($report:expr, lazy $severity:ident, $($rest:tt)*) => {
        $report.push(::report::ReportItem {
            lazy: true,
            severity: ::report::Severity::$severity,
            text: push!(@_format $($rest)*),
        })
    };
    ($report:expr, $severity:ident, $($rest:tt)*) => {
        $report.push(::report::ReportItem {
            lazy: false,
            severity: ::report::Severity::$severity,
            text: push!(@_format $($rest)*),
        })
    }
}

macro_rules! changed {
    ($report:expr, $severity:ident, $what:expr, ($was:expr => $now:expr) $($rest:tt)*) => {
        push!($report, $severity,
            concat!($what, " has changed:\n  Was: {:?}\n  Now: {:?}")
            $($rest)*, $was, $now
        )
    }
}

fn retain_mut<T, F>(vec: &mut Vec<T>, mut f: F)
    where F: FnMut(&mut T) -> bool
{
    let len = vec.len();
    let mut del = 0;
    {
        let v = &mut **vec;

        for i in 0..len {
            if !f(&mut v[i]) {
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
