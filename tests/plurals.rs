//! Catalog plural validation and rule selection.

use zi18n::{
    Catalog, CatalogError, Catalogs, Locale, Message, PluralCategory, PluralForm, TemplateError,
    cardinal_integer,
};

use PluralCategory::{Few, Many, One, Other, Two, Zero};

fn locale(tag: &str) -> Locale<'_> {
    Locale::parse(tag).unwrap()
}

#[test]
fn requires_other_and_rejects_duplicate_categories() {
    assert_eq!(
        Catalog::new(
            locale("en"),
            &[Message::plural(
                "files",
                &[PluralForm::new(One, "one file")]
            )],
        )
        .unwrap_err(),
        CatalogError::MissingOther("files")
    );
    assert_eq!(
        Catalog::new(locale("en"), &[Message::plural("empty", &[])]).unwrap_err(),
        CatalogError::MissingOther("empty")
    );
    assert_eq!(
        Catalog::new(
            locale("en"),
            &[Message::plural(
                "files",
                &[
                    PluralForm::new(Other, "files"),
                    PluralForm::new(Other, "more files")
                ],
            )],
        )
        .unwrap_err(),
        CatalogError::DuplicatePluralCategory {
            key: "files",
            category: Other
        }
    );
}

#[test]
fn validates_every_plural_template_before_lookup() {
    assert_eq!(
        Catalog::new(
            locale("en"),
            &[Message::plural(
                "files",
                &[PluralForm::new(Other, "files"), PluralForm::new(One, "{")]
            )],
        )
        .unwrap_err(),
        CatalogError::InvalidTemplate {
            key: "files",
            category: Some(One),
            error: TemplateError::UnclosedPlaceholder { position: 0 },
        }
    );
}

#[test]
fn selects_caller_categories_and_uses_other_for_absent_forms() {
    const FORMS: &[PluralForm<'static>] = &[
        PluralForm::new(Zero, "zero"),
        PluralForm::new(One, "one"),
        PluralForm::new(Two, "two"),
        PluralForm::new(Few, "few"),
        PluralForm::new(Other, "other"),
    ];
    const AR: &[Message<'static>] = &[Message::plural("files", FORMS)];
    let catalogs = [Catalog::new(locale("ar"), AR).unwrap()];
    let messages = Catalogs::new(&catalogs, &[]).unwrap();
    let matched = messages.lookup(locale("ar"), "files").unwrap();
    assert_eq!(matched.text(), None);
    for (category, expected) in [
        (Zero, "zero"),
        (One, "one"),
        (Two, "two"),
        (Few, "few"),
        (Many, "other"),
        (Other, "other"),
    ] {
        assert_eq!(matched.select_plural(category), Some(expected));
    }
}

#[test]
fn plural_provider_uses_the_actual_fallback_catalog() {
    const EN: &[Message<'static>] = &[Message::plural(
        "files",
        &[
            PluralForm::new(One, "{count} file"),
            PluralForm::new(Other, "{count} files"),
        ],
    )];
    const ZH: &[Message<'static>] = &[Message::text("label", "文件")];
    let catalogs = [
        Catalog::new(locale("eN"), EN).unwrap(),
        Catalog::new(locale("zh"), ZH).unwrap(),
    ];
    let fallbacks = [locale("en")];
    let messages = Catalogs::new(&catalogs, &fallbacks).unwrap();

    let matched = messages.lookup(locale("zh-HK"), "files").unwrap();
    assert_eq!(matched.locale().as_str(), "eN");
    assert_eq!(matched.select_cardinal_integer(1), Ok(Some("{count} file")));
    assert_eq!(
        matched.select_cardinal_integer(0),
        Ok(Some("{count} files"))
    );
    assert_eq!(
        matched.select_plural_with(|tag| {
            assert_eq!(tag.as_str(), "eN");
            Ok::<_, ()>(One)
        }),
        Ok(Some("{count} file"))
    );
    assert_eq!(
        matched.select_plural_with(|_| Err("rules error")),
        Err("rules error")
    );
    let matched = messages.lookup(locale("zh"), "label").unwrap();
    assert_eq!(matched.select_plural(One), None);
    assert_eq!(
        matched.select_plural_with::<()>(|_| panic!("called for text")),
        Ok(None)
    );
}

#[test]
fn built_in_cardinal_rules_preserve_u64_bounds_and_reject_other_languages() {
    for count in [0, 1, 2, u64::MAX - 1, u64::MAX] {
        let english = if count == 1 { One } else { Other };
        assert_eq!(cardinal_integer(locale("EN-GB"), count), Ok(english));
        assert_eq!(cardinal_integer(locale("zh-Hant-TW"), count), Ok(Other));
    }
    for tag in ["fr-FR", "eng", "x-en", "yue"] {
        let unsupported = cardinal_integer(locale(tag), 1).unwrap_err();
        assert_eq!(unsupported.locale.as_str(), tag);
    }
}

#[test]
fn unsupported_matched_locale_surfaces_after_fallback() {
    const FR: &[Message<'static>] = &[Message::plural(
        "files",
        &[PluralForm::new(Other, "fichiers")],
    )];
    let catalogs = [Catalog::new(locale("fr"), FR).unwrap()];
    let fallbacks = [locale("fr")];
    let messages = Catalogs::new(&catalogs, &fallbacks).unwrap();
    let error = messages
        .lookup(locale("en"), "files")
        .unwrap()
        .select_cardinal_integer(1)
        .unwrap_err();
    assert_eq!(error.locale.as_str(), "fr");
}
