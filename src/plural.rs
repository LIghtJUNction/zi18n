use crate::Locale;
use core::fmt;

/// A category selected by the caller's plural rules, or by [`cardinal_integer`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PluralCategory {
    /// The provider selected the `zero` category.
    Zero,
    /// The provider selected the `one` category.
    One,
    /// The provider selected the `two` category.
    Two,
    /// The provider selected the `few` category.
    Few,
    /// The provider selected the `many` category.
    Many,
    /// The provider selected `other`, also used when a category has no form.
    Other,
}

/// One category and its borrowed template in a plural message.
#[derive(Clone, Copy, Debug)]
pub struct PluralForm<'text> {
    pub(crate) category: PluralCategory,
    pub(crate) template: &'text str,
}

impl<'text> PluralForm<'text> {
    /// Defines a form to be validated during catalog construction.
    pub const fn new(category: PluralCategory, template: &'text str) -> Self {
        Self { category, template }
    }
}

/// Selects an unsigned integer cardinal category for primary languages `en` and `zh`.
///
/// English uses [`PluralCategory::One`] for exactly 1 and `Other` otherwise.
/// Chinese always uses `Other`. Subtags do not change these two rules.
/// Other primary languages return [`UnsupportedLocale`]. This helper accepts
/// plain integers only. For lookup results, use
/// [`crate::MatchedMessage::select_cardinal_integer`] to use the matched locale.
pub fn cardinal_integer(
    locale: Locale<'_>,
    count: u64,
) -> Result<PluralCategory, UnsupportedLocale<'_>> {
    let tag = locale.as_str();
    let language = tag.split_once('-').map_or(tag, |(first, _)| first);
    if language.eq_ignore_ascii_case("en") {
        return Ok(if count == 1 {
            PluralCategory::One
        } else {
            PluralCategory::Other
        });
    }
    if language.eq_ignore_ascii_case("zh") {
        return Ok(PluralCategory::Other);
    }
    Err(UnsupportedLocale { locale })
}

/// There is no built-in integer cardinal rule for this locale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct UnsupportedLocale<'tag> {
    /// The locale passed to the built-in rules.
    pub locale: Locale<'tag>,
}

impl fmt::Display for UnsupportedLocale<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "no integer cardinal rule for {}", self.locale)
    }
}

impl core::error::Error for UnsupportedLocale<'_> {}
