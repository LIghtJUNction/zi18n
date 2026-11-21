# zi18n

zi18n is Project Z's Rust library for borrowed localization catalogs, explicit language fallback, and named template formatting. The application supplies the messages, requested locale, and fallback order. The core has zero runtime dependencies and requires no allocator. Caller-supplied `Write` and `Display` implementations control their own allocations.

## Install

Add the registry release to your Cargo manifest.

```toml
[dependencies]
zi18n = { version = "=0.1.0", default-features = false }
```

[Release verification](https://github.com/LIghtJUNction/zi18n/blob/main/docs/releases/0.1.0.md) records the published package and registry-consumer checks.

## Example

The same catalog set can serve plain labels and plural counters. A missing Chinese key can fall back to English, and the returned message carries the locale of its actual catalog.

```rust
use zi18n::{Argument, Catalog, Catalogs, Locale, Message, PluralCategory, PluralForm, format_to};

const EN: &[Message<'static>] = &[
    Message::text("status", "Ready"),
    Message::text("retry", "Retry"),
    Message::plural("files", &[
        PluralForm::new(PluralCategory::One, "{count} file"),
        PluralForm::new(PluralCategory::Other, "{count} files"),
    ]),
];
const ZH: &[Message<'static>] = &[
    Message::text("status", "就绪"),
    Message::plural("files", &[
        PluralForm::new(PluralCategory::Other, "{count} 个文件"),
    ]),
];

let en = Locale::parse("en")?;
let zh = Locale::parse("zh")?;
let catalogs = [Catalog::new(en, EN)?, Catalog::new(zh, ZH)?];
let fallbacks = [en];
let messages = Catalogs::new(&catalogs, &fallbacks)?;
let requested = Locale::parse("zh-Hans-CN")?;

assert_eq!(messages.lookup(requested, "status")?.text(), Some("就绪"));
assert_eq!(messages.lookup(requested, "retry")?.locale(), en);

let matched = messages.lookup(requested, "files")?;
let count = 2_u64;
let template = matched.select_cardinal_integer(count)?.unwrap();
let mut output = String::new(); // The caller chooses the sink.
format_to(&mut output, template, &[Argument::new("count", &count)])?;
assert_eq!(matched.locale(), zh);
assert_eq!(output, "2 个文件");
# Ok::<(), Box<dyn std::error::Error>>(())
```

The [zmcl adapter example](examples/zmcl_adapter.rs) keeps the existing `--version` output in English and verifies the output bytes, including the newline.

## Features

| Cargo features | Available API |
| --- | --- |
| Default (`[]`) | `no_std` catalog lookup and formatting to a caller-supplied `core::fmt::Write`, with no allocator required |
| `alloc` | Adds `zi18n::format` to return an allocated `String` |
| `std` | Enables `alloc` and standard-library mode. Errors implement `core::error::Error` under all feature sets |

Rust 1.85.0 is the minimum supported version. The crate uses edition 2024.

## Locale and fallback

`Locale::parse` borrows the original tag and checks RFC 5646 structure, including extlangs, extensions, private use, and the fixed grandfathered tags. It rejects duplicate variants and extension singletons. Locale comparisons ignore ASCII case. Registry validity, alias canonicalization, extension semantics, likely subtags, and OS locale discovery are outside this API. [RFC 5646 §2.1](https://www.rfc-editor.org/rfc/rfc5646.html#section-2.1), [§2.2.9](https://www.rfc-editor.org/rfc/rfc5646.html#section-2.2.9)

Lookup searches **per key** in the exact request, its truncated parents, then the caller's configured fallback locales in order. The search skips absent request candidates and absent keys, returns the matched catalog's original locale spelling, and produces `MissingMessage` when it runs out of candidates.

Each configured fallback must name an exact catalog, ignoring ASCII case. `Catalogs::new` returns `CatalogError::UnknownFallback` with the zero-based index of the first missing catalog. Pass `&[]` for no configured fallback. The search uses the list as supplied, so include any desired fallback parent explicitly.

Parent truncation removes the last subtag and all consecutive single-character subtags immediately before it, including private-use values. For example, `en-x-a-tail` goes directly to `en`, skipping the valid tag `en-x-a`. The request `zh-Hant-HK-u-nu-hanidec` tries `zh-Hant-HK-u-nu`, then `zh-Hant-HK`, `zh-Hant`, and `zh`. This is an exact-catalog policy with no Accept-Language parsing or likely-subtag inference such as `zh-HK` to `zh-Hant`. [RFC 4647 §3.4](https://www.rfc-editor.org/rfc/rfc4647.html#section-3.4)

## Catalogs and formatting

`Catalog::new` checks all templates, duplicate keys, duplicate plural categories, and the presence of `Other` in every plural message. `Catalogs::new` rejects duplicates across catalog locales. Message keys are case-sensitive.

Templates support `{name}`, `{{` for a literal opening brace, and `}}` for a literal closing brace. Argument names are case-sensitive ASCII identifiers matching `[A-Za-z_][A-Za-z0-9_]*`. Values can implement `Display` and their output is opaque, including any braces. Valid, unique extra arguments are accepted and ignored. Repeated placeholders reuse the same argument.

Format syntax and argument validation occur before any sink writes or value formatting. Syntax errors report a UTF-8 byte offset, and argument errors report the affected name. A sink or replacement `Display` failure returns `FormatError::Write` and can leave a partial prefix.

## Plurals

The caller can select any of the six categories directly or pass a rules provider to `select_plural_with`. The provider receives the **matched catalog locale**, including when lookup fell back. An absent category uses the catalog's required `Other` form.

`cardinal_integer` handles only English (`en`) and Chinese (`zh`) primary languages and accepts the full `u64` range. English selects `One` for exactly 1 and `Other` otherwise, while Chinese always selects `Other`. Other languages return `UnsupportedLocale`. The helper excludes visible fraction digits, compact exponents, ordinals, and localized number formatting. [UTS #35 rev. 79](https://www.unicode.org/reports/tr35/tr35-79/tr35-numbers.html#Language_Plural_Rules), [CLDR 49 cardinal data](https://github.com/unicode-org/cldr/blob/96e54b273d2da4ec6fbad350ba19ff23fa24a588/common/supplemental/plurals.xml)

[Design notes](docs/design.md) describe the validation boundaries and how the operations use borrowed data.
