//! Formatting grammar, preflight, and streaming output contracts.

use std::cell::Cell;
use std::fmt::{self, Write};
use zi18n::{Argument, FormatError, TemplateError, format_to};

#[test]
fn unicode_escapes_repeated_placeholders_and_opaque_replacements() {
    let mut output = String::new();
    let sink: &mut dyn Write = &mut output;
    format_to(
        sink,
        "用户 {{{name}}}：{name}；{{literal}}，{_n2}",
        &[
            Argument::new("name", &"称呼 {secret}}"),
            Argument::new("_n2", &u64::MAX),
        ],
    )
    .unwrap();
    assert_eq!(
        output,
        "用户 {称呼 {secret}}}：称呼 {secret}}；{literal}，18446744073709551615"
    );
}

#[test]
fn malformed_placeholders_return_utf8_byte_offsets() {
    for (template, error) in [
        (
            "前缀 {open",
            TemplateError::UnclosedPlaceholder { position: 7 },
        ),
        ("é }", TemplateError::UnexpectedClosingBrace { position: 3 }),
        ("前缀 {}", TemplateError::InvalidPlaceholder { position: 7 }),
        ("{0}", TemplateError::InvalidPlaceholder { position: 0 }),
        ("{a b}", TemplateError::InvalidPlaceholder { position: 0 }),
        ("{a-b}", TemplateError::InvalidPlaceholder { position: 0 }),
        ("{汉字}", TemplateError::InvalidPlaceholder { position: 0 }),
        ("{a{b}", TemplateError::InvalidPlaceholder { position: 0 }),
        ("{{}", TemplateError::UnexpectedClosingBrace { position: 2 }),
    ] {
        let mut output = String::new();
        assert_eq!(
            format_to(&mut output, template, &[]),
            Err(FormatError::InvalidTemplate(error))
        );
        assert!(output.is_empty());
    }
}

struct DisplayProbe<'probe>(&'probe Cell<usize>);

impl fmt::Display for DisplayProbe<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.set(self.0.get() + 1);
        f.write_str("probe")
    }
}

#[derive(Default)]
struct WriteProbe {
    calls: usize,
}

impl Write for WriteProbe {
    fn write_str(&mut self, _: &str) -> fmt::Result {
        self.calls += 1;
        Ok(())
    }
}

#[test]
fn preflight_failures_write_nothing_and_do_not_call_display() {
    let calls = Cell::new(0);
    let probe = DisplayProbe(&calls);
    let cases: [(&str, &[Argument<'_, '_>], FormatError<'_>); 6] = [
        (
            "a prefix {name} malformed }",
            &[Argument::new("name", &probe)],
            FormatError::InvalidTemplate(TemplateError::UnexpectedClosingBrace { position: 26 }),
        ),
        (
            "a prefix {name} then {missing}",
            &[Argument::new("name", &probe)],
            FormatError::MissingArgument("missing"),
        ),
        (
            "a prefix {name} then {Name}",
            &[Argument::new("name", &probe)],
            FormatError::MissingArgument("Name"),
        ),
        (
            "a prefix {name}",
            &[
                Argument::new("name", &probe),
                Argument::new("bad-name", &probe),
            ],
            FormatError::InvalidArgumentName("bad-name"),
        ),
        (
            "a prefix {name}",
            &[Argument::new("name", &probe), Argument::new("name", &probe)],
            FormatError::DuplicateArgument("name"),
        ),
        (
            "plain",
            &[
                Argument::new("extra", &probe),
                Argument::new("extra", &probe),
            ],
            FormatError::DuplicateArgument("extra"),
        ),
    ];
    for (template, arguments, error) in cases {
        let mut sink = WriteProbe::default();
        assert_eq!(format_to(&mut sink, template, arguments), Err(error));
        assert_eq!(sink.calls, 0);
        assert_eq!(calls.get(), 0);
    }
}

#[test]
fn valid_extra_arguments_are_ignored_and_names_are_case_sensitive() {
    let calls = Cell::new(0);
    let extra = DisplayProbe(&calls);
    let mut output = String::new();
    format_to(
        &mut output,
        "{name}:{Name}",
        &[
            Argument::new("NAME", &extra),
            Argument::new("Name", &"乙"),
            Argument::new("name", &"甲"),
        ],
    )
    .unwrap();
    assert_eq!(output, "甲:乙");
    assert_eq!(calls.get(), 0);
}

#[test]
fn a_failing_sink_may_keep_a_partial_prefix() {
    struct Limited(String);
    impl Write for Limited {
        fn write_str(&mut self, text: &str) -> fmt::Result {
            if self.0.len() + text.len() > 6 {
                return Err(fmt::Error);
            }
            self.0.write_str(text)
        }
    }

    let mut output = Limited(String::new());
    assert_eq!(
        format_to(
            &mut output,
            "start {name} remainder",
            &[Argument::new("name", &"a name")]
        ),
        Err(FormatError::Write)
    );
    assert_eq!(output.0, "start ");
}

#[test]
fn a_failing_display_may_keep_a_partial_prefix() {
    struct BadDisplay;
    impl fmt::Display for BadDisplay {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("{opaque}")?;
            Err(fmt::Error)
        }
    }

    let mut output = String::new();
    assert_eq!(
        format_to(
            &mut output,
            "start {name} end",
            &[Argument::new("name", &BadDisplay)]
        ),
        Err(FormatError::Write)
    );
    assert_eq!(output, "start {opaque}");
}

#[cfg(feature = "alloc")]
#[test]
fn alloc_format_shares_rendering_and_preflight() {
    let arguments = [Argument::new("who", &"名字 {other}")];
    let mut via_sink = String::new();
    format_to(&mut via_sink, "{{{who}}}", &arguments).unwrap();
    assert_eq!(zi18n::format("{{{who}}}", &arguments), Ok(via_sink));
    assert_eq!(
        zi18n::format("{absent}", &arguments),
        Err(FormatError::MissingArgument("absent"))
    );
}

#[cfg(feature = "std")]
#[test]
fn std_error_chain_exposes_template_error() {
    use std::error::Error;

    let error = zi18n::format("标签}", &[]).unwrap_err();
    assert_eq!(
        error.source().unwrap().to_string(),
        "unexpected closing brace at byte 6"
    );
}
