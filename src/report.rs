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
    pub severity: Severity,
    pub text: Cow<'static, str>,
}

impl ReportItem {
    pub fn new<S: Into<Cow<'static, str>>>(severity: Severity, message: S) -> ReportItem {
        ReportItem { severity: severity, text: message.into() }
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

    pub fn push(&mut self, child: ReportItem) {
        self.children.push(child.into());
    }

    pub fn nest(&mut self, child: ReportItem) -> &mut Report {
        self.children.push(child.into());
        self.children.last_mut().unwrap()
    }

    pub fn highest_severity(&self) -> Severity {
        self.children.iter().map(Report::highest_severity).max().unwrap_or(self.item.severity)
    }
}

impl From<ReportItem> for Report {
    fn from(val: ReportItem) -> Report {
        Report { item: val, children: Vec::new() }
    }
}

macro_rules! push {
    ($report:expr, $severity:ident, $($rest:tt)*) => {
        $report.nest(::report::ReportItem {
            severity: ::report::Severity::$severity,
            text: format!($($rest)*).into(),
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
