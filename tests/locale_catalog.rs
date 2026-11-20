//! Locale, catalog construction, and message lookup contracts.

use zi18n::{Catalog, CatalogError, Catalogs, Locale, LocaleError, Message, TemplateError};

fn locale(tag: &str) -> Locale<'_> {
    Locale::parse(tag).unwrap()
}

#[test]
fn accepts_structural_language_tags_and_preserves_spelling() {
    let tags = [
        "en",
        "eN-LaTn-uS",
        "zh-Hant-TW",
        "de-419",
        "qaaa-Latn",
        "abcdefgh",
        "zh-cmn-Hans-CN",
        "zh-aaa-bbb-ccc-Hans-CN",
        "sl-rozaj-biske-1994",
        "de-DE-u-co-phonebk",
        "en-0-alpha-a-bravo-Z-charlie",
        "en-x-a-a",
        "x-project-7",
        "i-KLINGON",
        "en-GB-oed",
        "sgn-BE-FR",
        "zh-min-nan",
    ];
    for tag in tags {
        assert_eq!(locale(tag).as_str(), tag, "{tag}");
        let mut current = locale(tag);
        while let Some(parent) = current.parent() {
            assert_eq!(Locale::parse(parent.as_str()), Ok(parent));
            assert!(parent.as_str().len() < current.as_str().len());
            current = parent;
        }
    }
    assert_eq!(locale("en-Latn-US"), locale("EN-latn-us"));
    assert_ne!(locale("i-KLINGON"), locale("tlh"));
}

#[test]
fn rejects_malformed_language_tags() {
    for tag in [
        "",
        "-",
        "e",
        "123",
        "en_",
        "en--US",
        "-en",
        "en-",
        "中文",
        "en-ñ",
        "en-US.UTF-8",
        "*",
        "en-123456789",
        "abcdefghi",
        "xx-US-Latn",
        "en-US-GB",
        "zh-aaa-bbb-ccc-ddd",
        "abcd-abc",
        "en-abc1",
        "x",
        "en-x",
        "en-u",
        "en-9",
        "en-a-x-test",
        "en-u-a-test",
        "i-notknown",
        "en-GB-oed-x-test",
    ] {
        assert_eq!(Locale::parse(tag), Err(LocaleError::InvalidSyntax), "{tag}");
    }
}

#[test]
fn rejects_case_insensitive_duplicate_variants_and_extension_singletons() {
    assert_eq!(
        Locale::parse("sl-rozaj-biske-ROZAJ"),
        Err(LocaleError::DuplicateVariant)
    );
    assert_eq!(
        Locale::parse("en-1901-1901"),
        Err(LocaleError::DuplicateVariant)
    );
    assert_eq!(
        Locale::parse("en-a-demo-u-xy-A-next"),
        Err(LocaleError::DuplicateExtension)
    );
    assert_eq!(
        Locale::parse("en-7-demo-7-next"),
        Err(LocaleError::DuplicateExtension)
    );
}

#[test]
fn truncates_extensions_private_use_and_grandfathered_tags() {
    for (tag, parents) in [
        ("de-DE-u-co-phonebk", &["de-DE-u-co", "de-DE", "de"][..]),
        (
            "zh-Hant-CN-x-private1-private2",
            &["zh-Hant-CN-x-private1", "zh-Hant-CN", "zh-Hant", "zh"],
        ),
        (
            "en-7-demo-a-xy-x-a-tail",
            &["en-7-demo-a-xy", "en-7-demo", "en"],
        ),
        ("en-x-a-b-long", &["en"]),
        ("x-alpha-7", &["x-alpha"]),
        ("x-a-b", &[]),
        ("i-klingon", &[]),
        ("zh-min-nan", &["zh-min", "zh"]),
        ("en-GB-oed", &["en-GB", "en"]),
    ] {
        let actual: Vec<_> = std::iter::successors(locale(tag).parent(), |tag| tag.parent())
            .map(Locale::as_str)
            .collect();
        assert_eq!(actual, parents, "{tag}");
    }
}

#[test]
fn looks_up_each_key_through_exact_parent_and_ordered_fallback_catalogs() {
    const CATALOGS: &[(&str, &[Message<'static>])] = &[
        ("ZH-hant", &[Message::text("menu", "選單")]),
        ("zh-Hant-HK", &[Message::text("region", "香港")]),
        ("zh", &[Message::text("name", "中文")]),
        ("en", &[Message::text("hint", "Use Enter")]),
        (
            "en-GB",
            &[
                Message::text("hint", "Press Enter"),
                Message::text("colours", "Colours"),
            ],
        ),
        ("fr", &[Message::text("quit", "Quitter")]),
    ];
    let catalogs: Vec<_> = CATALOGS
        .iter()
        .map(|&(tag, definitions)| Catalog::new(locale(tag), definitions).unwrap())
        .collect();
    let fallback = [locale("en-gb"), locale("fr"), locale("en")];
    let messages = Catalogs::new(&catalogs, &fallback).unwrap();
    let request = locale("ZH-hant-HK");
    let cases = [
        ("region", "zh-Hant-HK", "香港"),
        ("menu", "ZH-hant", "選單"),
        ("name", "zh", "中文"),
        ("colours", "en-GB", "Colours"),
        ("hint", "en-GB", "Press Enter"),
        ("quit", "fr", "Quitter"),
    ];
    for (key, matched, text) in cases {
        let result = messages.lookup(request, key).unwrap();
        assert_eq!(result.locale().as_str(), matched);
        assert_eq!(result.message().key(), key);
        assert_eq!(result.text(), Some(text));
    }
    let error = messages.lookup(request, "Menu").unwrap_err();
    assert_eq!(error.requested.as_str(), "ZH-hant-HK");
    assert_eq!(error.key, "Menu");
    assert!(messages.lookup(locale("zh-HK"), "menu").is_err());
}

#[test]
fn rejects_configured_fallbacks_without_an_exact_catalog() {
    let catalogs = [Catalog::new(locale("en"), &[]).unwrap()];
    assert_eq!(
        Catalogs::new(&catalogs, &[locale("EN"), locale("pt-BR")]).unwrap_err(),
        CatalogError::UnknownFallback { index: 1 }
    );
    assert_eq!(
        Catalogs::new(&catalogs, &[locale("en-GB")]).unwrap_err(),
        CatalogError::UnknownFallback { index: 0 }
    );
}

#[test]
fn configured_fallback_checks_exact_catalog_only() {
    const EN: &[Message<'static>] = &[Message::text("hello", "Hello")];
    const EN_GB: &[Message<'static>] = &[Message::text("colours", "Colours")];
    let catalogs = [
        Catalog::new(locale("en"), EN).unwrap(),
        Catalog::new(locale("en-GB"), EN_GB).unwrap(),
    ];
    let fallback = [locale("en-GB")];
    let messages = Catalogs::new(&catalogs, &fallback).unwrap();
    let request = locale("fr");
    assert_eq!(
        messages.lookup(request, "colours").unwrap().locale(),
        fallback[0]
    );
    assert!(messages.lookup(request, "hello").is_err());
    assert_eq!(
        messages.lookup(locale("en-GB"), "hello").unwrap().text(),
        Some("Hello")
    );
    let no_fallback = Catalogs::new(&catalogs, &[]).unwrap();
    assert!(no_fallback.lookup(request, "colours").is_err());
}

#[test]
fn lookup_crosses_extension_and_private_use_singletons() {
    const SORT: &[Message<'static>] = &[Message::text("sort", "Sortierung")];
    const REGION: &[Message<'static>] = &[Message::text("region", "Deutschland")];
    const PRIVATE: &[Message<'static>] = &[Message::text("name", "Local")];
    let catalogs = [
        Catalog::new(locale("de-DE-u-co"), SORT).unwrap(),
        Catalog::new(locale("de-DE"), REGION).unwrap(),
        Catalog::new(locale("x-local"), PRIVATE).unwrap(),
    ];
    let messages = Catalogs::new(&catalogs, &[]).unwrap();
    let request = locale("DE-de-u-co-phonebk-x-a-test");
    assert_eq!(
        messages.lookup(request, "sort").unwrap().locale(),
        locale("de-DE-u-co")
    );
    assert_eq!(
        messages.lookup(request, "region").unwrap().locale(),
        locale("de-DE")
    );
    assert_eq!(
        messages.lookup(locale("x-local-a"), "name").unwrap().text(),
        Some("Local")
    );
}

#[test]
fn rejects_duplicate_catalog_locales_and_keys_but_allows_case_distinct_keys() {
    let catalogs = [
        Catalog::new(locale("zh-Hans"), &[]).unwrap(),
        Catalog::new(locale("ZH-hans"), &[]).unwrap(),
    ];
    assert!(matches!(
        Catalogs::new(&catalogs, &[]),
        Err(CatalogError::DuplicateLocale(tag)) if tag.as_str() == "ZH-hans"
    ));
    assert_eq!(
        Catalog::new(
            locale("en"),
            &[Message::text("same", "one"), Message::text("same", "two")]
        )
        .unwrap_err(),
        CatalogError::DuplicateKey("same")
    );
    assert!(
        Catalog::new(
            locale("en"),
            &[Message::text("same", "one"), Message::text("SAME", "two")]
        )
        .is_ok()
    );
}

#[test]
fn invalid_text_template_is_rejected_at_construction() {
    let definitions = [Message::text("label", "标签}")];
    let error = Catalog::new(locale("zh"), &definitions).unwrap_err();
    assert_eq!(
        error,
        CatalogError::InvalidTemplate {
            key: "label",
            category: None,
            error: TemplateError::UnexpectedClosingBrace { position: 6 },
        }
    );
}

#[test]
fn borrows_runtime_input_and_supports_empty_translations() {
    let tag = String::from("en");
    let key = String::from("dynamic");
    let template = String::new();
    let definitions = [Message::text(&key, &template)];
    let catalogs = [Catalog::new(locale(&tag), &definitions).unwrap()];
    let messages = Catalogs::new(&catalogs, &[]).unwrap();
    let matched = {
        let request = String::from("EN");
        let query = String::from("dynamic");
        messages.lookup(locale(&request), &query).unwrap()
    };
    assert_eq!(matched.text(), Some(""));
    assert!(std::ptr::eq(matched.message(), &definitions[0]));
}
