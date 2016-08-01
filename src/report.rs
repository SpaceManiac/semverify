//! Report

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
    SemiBreaking,
    /// Change requiring bumping of major release number
    Breaking,
    /// Tool error
    Error,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ReportItem {
    pub severity: Severity,
    pub text: String,
}

#[derive(Debug)]
pub struct Report {
    pub items: Vec<ReportItem>,
}

impl Report {
    pub fn new() -> Report {
        Report { items: Vec::new() }
    }

    pub fn max_severity(&self) -> Severity {
        self.items.iter().map(|i| i.severity).max().unwrap_or(Severity::Note)
    }
}

macro_rules! push {
    ($report:expr, $severity:ident, $($rest:tt)*) => {
        $report.items.push(::report::ReportItem {
            severity: ::report::Severity::$severity,
            text: format!($($rest)*),
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
