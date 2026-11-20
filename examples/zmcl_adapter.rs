//! Thin application adapter for the existing English `zmcl --version` line.

use std::fmt::{self, Write};
use zi18n::{Argument, Catalog, Catalogs, Locale, Message, format_to};

const ENGLISH: &[Message<'static>] = &[Message::text("cli.version", "zmcl {version}\n")];

fn write_version(output: &mut impl Write, version: &str) -> fmt::Result {
    let en = Locale::parse("en").expect("fixed English locale");
    let catalogs = [Catalog::new(en, ENGLISH).expect("valid English catalog")];
    let messages = Catalogs::new(&catalogs, &[]).expect("one catalog");
    let message = messages.lookup(en, "cli.version").expect("version message");
    let template = message.text().expect("text template");
    format_to(output, template, &[Argument::new("version", &version)]).map_err(|_| fmt::Error)
}

fn main() -> fmt::Result {
    let version = "1.2.3"; // zmcl supplies env!("CARGO_PKG_VERSION") at its call site.
    let mut output = String::new();
    write_version(&mut output, version)?;
    print!("{output}");
    Ok(())
}

#[test]
fn preserves_english_version_line() {
    let version = "1.2.3";
    let mut output = String::new();
    write_version(&mut output, version).unwrap();
    assert_eq!(output.as_bytes(), format!("zmcl {version}\n").as_bytes());
}
