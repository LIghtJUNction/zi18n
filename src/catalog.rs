use crate::template::validate_template;
use crate::{
    Locale, PluralCategory, PluralForm, TemplateError, UnsupportedLocale, cardinal_integer,
};
use core::fmt;

/// A borrowed message definition suitable for a static array or runtime data.
#[derive(Clone, Copy, Debug)]
pub struct Message<'data> {
    key: &'data str,
    body: Body<'data>,
}

#[derive(Clone, Copy, Debug)]
enum Body<'data> {
    Text(&'data str),
    Plural(&'data [PluralForm<'data>]),
}

impl<'data> Message<'data> {
    /// Defines a text template to be checked by [`Catalog::new`].
    pub const fn text(key: &'data str, template: &'data str) -> Self {
        Self {
            key,
            body: Body::Text(template),
        }
    }

    /// Defines plural forms to be checked by [`Catalog::new`].
    pub const fn plural(key: &'data str, forms: &'data [PluralForm<'data>]) -> Self {
        Self {
            key,
            body: Body::Plural(forms),
        }
    }

    /// Returns the case-sensitive lookup key.
    pub const fn key(&self) -> &'data str {
        self.key
    }
}

/// A locale and its validated messages.
///
/// Message keys are opaque, case-sensitive strings.
#[derive(Clone, Copy, Debug)]
pub struct Catalog<'data> {
    locale: Locale<'data>,
    messages: &'data [Message<'data>],
}

impl<'data> Catalog<'data> {
    /// Checks for duplicate keys, invalid templates, and invalid plural forms.
    pub fn new(
        locale: Locale<'data>,
        messages: &'data [Message<'data>],
    ) -> Result<Self, CatalogError<'data>> {
        for (index, message) in messages.iter().enumerate() {
            let key = message.key;
            if messages[..index].iter().any(|prior| prior.key == key) {
                return Err(CatalogError::DuplicateKey(key));
            }
            match message.body {
                Body::Text(template) => check_template(key, None, template)?,
                Body::Plural(forms) => {
                    let mut has_other = false;
                    for (index, form) in forms.iter().enumerate() {
                        let category = form.category;
                        if forms[..index]
                            .iter()
                            .any(|prior| prior.category == category)
                        {
                            return Err(CatalogError::DuplicatePluralCategory { key, category });
                        }
                        check_template(key, Some(category), form.template)?;
                        has_other |= category == PluralCategory::Other;
                    }
                    if !has_other {
                        return Err(CatalogError::MissingOther(key));
                    }
                }
            }
        }
        Ok(Self { locale, messages })
    }

    /// Returns the locale declared by this catalog.
    pub const fn locale(&self) -> Locale<'data> {
        self.locale
    }
}

/// A validated set of catalogs and an explicit sequence of fallback locales.
#[derive(Clone, Copy, Debug)]
pub struct Catalogs<'tables, 'data> {
    catalogs: &'tables [Catalog<'data>],
    fallbacks: &'tables [Locale<'tables>],
}

impl<'tables, 'data> Catalogs<'tables, 'data> {
    /// Rejects duplicate catalog locales, compared case-insensitively.
    ///
    /// Configured fallbacks are exact catalog locales in priority order.
    /// A fallback without a catalog is skipped.
    pub fn new(
        catalogs: &'tables [Catalog<'data>],
        fallbacks: &'tables [Locale<'tables>],
    ) -> Result<Self, CatalogError<'data>> {
        for (index, catalog) in catalogs.iter().enumerate() {
            if catalogs[..index]
                .iter()
                .any(|prior| prior.locale == catalog.locale)
            {
                return Err(CatalogError::DuplicateLocale(catalog.locale));
            }
        }
        Ok(Self {
            catalogs,
            fallbacks,
        })
    }

    /// Looks up a key in the request, then its parents, then each configured fallback.
    ///
    /// A catalog without the key does not end the search. Fallback locales are checked
    /// exactly, in order, without further parent expansion.
    pub fn lookup<'query>(
        &self,
        requested: Locale<'query>,
        key: &'query str,
    ) -> Result<MatchedMessage<'data>, MissingMessage<'query>> {
        let requested_locales = core::iter::successors(Some(requested), |locale| locale.parent());
        for locale in requested_locales.chain(self.fallbacks.iter().copied()) {
            if let Some(catalog) = self
                .catalogs
                .iter()
                .find(|catalog| catalog.locale == locale)
            {
                if let Some(message) = catalog.messages.iter().find(|message| message.key == key) {
                    return Ok(MatchedMessage {
                        locale: catalog.locale,
                        message,
                    });
                }
            }
        }
        Err(MissingMessage { requested, key })
    }
}

/// A message paired with the actual catalog locale used to resolve it.
#[derive(Clone, Copy, Debug)]
pub struct MatchedMessage<'data> {
    locale: Locale<'data>,
    message: &'data Message<'data>,
}

impl<'data> MatchedMessage<'data> {
    /// Returns the matched catalog locale, including its original spelling.
    pub const fn locale(self) -> Locale<'data> {
        self.locale
    }

    /// Returns the message definition from the matched catalog.
    pub const fn message(self) -> &'data Message<'data> {
        self.message
    }

    /// Returns the template for a text message. Plural messages return `None`.
    ///
    /// A plain label can be used directly. Render braces and placeholders with
    /// [`crate::format_to`].
    pub const fn text(self) -> Option<&'data str> {
        match self.message.body {
            Body::Text(template) => Some(template),
            Body::Plural(_) => None,
        }
    }

    /// Returns the selected template, using `Other` when the category is absent.
    ///
    /// Text messages return `None`. If a rules provider needs a locale, use
    /// [`Self::select_plural_with`] to pass it the actual matched catalog locale.
    pub fn select_plural(self, category: PluralCategory) -> Option<&'data str> {
        let Body::Plural(forms) = self.message.body else {
            return None;
        };
        forms
            .iter()
            .find(|form| form.category == category)
            .or_else(|| {
                forms
                    .iter()
                    .find(|form| form.category == PluralCategory::Other)
            })
            .map(|form| form.template)
    }

    /// Calls the provider with the actual matched locale to select a plural template.
    ///
    /// The provider can capture any operands it needs. Its error is propagated.
    /// Text messages return `Ok(None)` without calling the provider.
    pub fn select_plural_with<E>(
        self,
        rules: impl FnOnce(Locale<'data>) -> Result<PluralCategory, E>,
    ) -> Result<Option<&'data str>, E> {
        match self.message.body {
            Body::Text(_) => Ok(None),
            Body::Plural(_) => Ok(self.select_plural(rules(self.locale)?)),
        }
    }

    /// Selects a plural template with the built-in [`cardinal_integer`] rules.
    ///
    /// The rules use the actual matched locale. Text messages return `Ok(None)`.
    pub fn select_cardinal_integer(
        self,
        count: u64,
    ) -> Result<Option<&'data str>, UnsupportedLocale<'data>> {
        self.select_plural_with(|locale| cardinal_integer(locale, count))
    }
}

/// A catalog or catalog-set validation error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum CatalogError<'data> {
    /// Two catalogs declare the same locale, ignoring ASCII case.
    DuplicateLocale(Locale<'data>),
    /// A key occurs more than once in one catalog.
    DuplicateKey(&'data str),
    /// A plural category occurs more than once for the key.
    DuplicatePluralCategory {
        /// The affected message key.
        key: &'data str,
        /// The repeated category.
        category: PluralCategory,
    },
    /// A plural message is missing the required `Other` form.
    MissingOther(&'data str),
    /// A text or plural template has invalid syntax.
    InvalidTemplate {
        /// The affected message key.
        key: &'data str,
        /// The form's category, or `None` for a text message.
        category: Option<PluralCategory>,
        /// The error within the template.
        error: TemplateError,
    },
}

impl fmt::Display for CatalogError<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateLocale(locale) => write!(formatter, "duplicate catalog locale {locale}"),
            Self::DuplicateKey(key) => write!(formatter, "duplicate message key {key:?}"),
            Self::DuplicatePluralCategory { key, category } => {
                write!(formatter, "duplicate {category:?} form for {key:?}")
            }
            Self::MissingOther(key) => write!(formatter, "missing Other form for {key:?}"),
            Self::InvalidTemplate {
                key,
                category,
                error,
            } => {
                write!(formatter, "invalid template for {key:?}")?;
                if let Some(category) = category {
                    write!(formatter, " ({category:?})")?;
                }
                write!(formatter, ": {error}")
            }
        }
    }
}

impl core::error::Error for CatalogError<'_> {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::InvalidTemplate { error, .. } => Some(error),
            _ => None,
        }
    }
}

/// No catalog in the configured lookup sequence contains the requested key.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub struct MissingMessage<'query> {
    /// The locale originally requested.
    pub requested: Locale<'query>,
    /// The key missing from all candidates.
    pub key: &'query str,
}

impl fmt::Display for MissingMessage<'_> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "missing message {:?} for {}",
            self.key, self.requested
        )
    }
}

impl core::error::Error for MissingMessage<'_> {}

fn check_template<'data>(
    key: &'data str,
    category: Option<PluralCategory>,
    template: &str,
) -> Result<(), CatalogError<'data>> {
    validate_template(template).map_err(|error| CatalogError::InvalidTemplate {
        key,
        category,
        error,
    })
}
