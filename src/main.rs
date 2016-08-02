extern crate semcmp;

use semcmp::report::{Report, ReportItem};

fn main() {
    let report = semcmp::create_report("inputs/old.rs".as_ref(), "inputs/new.rs".as_ref());
    print_report(0, &report);
}

fn print_report(indent: usize, report: &Report) {
    print_item(indent, &report.item);
    for child in &report.children {
        print_report(indent + 2, &child);
    }
}

fn print_item(indent: usize, item: &ReportItem) {
    let indent_str = format!("\n{:1$}", "", indent);
    println!("{}[{:?}] {}", &indent_str[1..],
        item.severity,
        item.text.replace("\n", &indent_str));
}
