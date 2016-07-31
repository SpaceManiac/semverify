use std::fmt;

#[derive(Copy, Clone)]
pub struct Indent(usize);

impl Indent {
    pub fn new() -> Indent {
        Indent(0)
    }
    pub fn next(self) -> Indent {
        Indent(self.0 + 1)
    }
}

const BLANK: &'static str = "                    ";

impl fmt::Display for Indent {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        let amt = self.0 * 2;
        for _ in 0..amt / BLANK.len() {
            try!(fmt.write_str(BLANK));
        }
        fmt.write_str(&BLANK[..amt % BLANK.len()])
    }
}
