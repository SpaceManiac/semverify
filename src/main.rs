extern crate semcmp;

fn main() {
    let report = semcmp::create_report("inputs/old.rs".as_ref(), "inputs/new.rs".as_ref());
    for item in &report.items {
        println!("{:?}: {}", item.severity, item.text);
    }
}
