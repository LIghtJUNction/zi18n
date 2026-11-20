#![cfg_attr(not(feature = "std"), no_std)]
#![doc = include_str!("../README.md")]

#[cfg(feature = "alloc")]
extern crate alloc;

mod catalog;
mod locale;
mod plural;
mod template;

pub use catalog::{Catalog, CatalogError, Catalogs, MatchedMessage, Message, MissingMessage};
pub use locale::{Locale, LocaleError};
pub use plural::{PluralCategory, PluralForm, UnsupportedLocale, cardinal_integer};
#[cfg(feature = "alloc")]
pub use template::format;
pub use template::{Argument, FormatError, TemplateError, format_to};
