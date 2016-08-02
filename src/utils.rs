use std::fmt;

pub fn parse_crate(file: &::std::path::Path) -> Option<::syntax::ast::Crate> {
    use std::rc::Rc;
    use syntax::codemap;
    use syntax::parse::parser::Parser;
    use syntax::parse::{lexer, ParseSess};
    use syntax::errors::Handler;
    use syntax::errors::emitter::ColorConfig;

    let cm = Rc::new(codemap::CodeMap::new());
    let sh = Handler::with_tty_emitter(ColorConfig::Never, None, false, false, Some(cm.clone()));
    let ps = ParseSess::with_span_handler(sh, cm);
    let fm = ps.codemap().load_file(file).unwrap();
    let srdr = lexer::StringReader::new(&ps.span_diagnostic, fm);
    let mut p = Parser::new(&ps, Vec::new(), Box::new(srdr));
    // who knows why this is needed
    (|p: &mut Parser| p.parse_crate_mod().ok())(&mut p)
}

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

impl fmt::Display for Indent {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        write!(fmt, "{:1$}", "", 2 * self.0)
    }
}
