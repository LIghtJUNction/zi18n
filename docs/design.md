# Design

zi18n separates catalog validation, message lookup, and rendering. An application can validate borrowed catalog data once and reuse it for different requests and output sinks.

## Borrowed data

`Locale` retains a borrowed spelling. `Message` and `PluralForm` are constant definitions, so an application can place them in static slices or borrow runtime-loaded data. `Catalog` validates each message and `Catalogs` validates the locale set. Both keep their slices immutable for the lifetime of the validated values. Data loading and language selection belong to the caller.

These types have no runtime dependencies and require no allocator. Validation uses scans of the supplied slices. Lookup performs an exact locale scan for each candidate and an exact key scan within a matching catalog. The library builds no index and keeps no shared mutable state.

## Locale candidates

The parser follows the language-tag productions in RFC 5646 §2.1 and checks variant and extension singleton uniqueness. It accepts syntactically valid unregistered combinations. A grandfathered spelling is retained as supplied. [RFC 5646 §2.1](https://www.rfc-editor.org/rfc/rfc5646.html#section-2.1), [§2.2.9](https://www.rfc-editor.org/rfc/rfc5646.html#section-2.2.9)

Lookup truncates the request according to the singleton rule from RFC 4647 §3.4. A shorter candidate may keep a shorter extension or private-use sequence. Every lookup starts from the full request for the specific key. [RFC 4647 §3.4](https://www.rfc-editor.org/rfc/rfc4647.html#section-3.4)

The fallback list supplies exact catalog locales in priority order after the requested parents. `Catalogs::new` checks each entry against the catalog set case-insensitively and reports the first unknown index. Only the requested locale expands to its parents. An empty list ends lookup after the requested hierarchy.

## Rendering

A single tokenizer interprets UTF-8 text, ASCII braces, and argument names. Catalog validation uses it to check grammar. Standalone formatting uses it to preflight the template and its arguments, then walks the same template again to render. The argument list and template are borrowed in both passes. Replacement values go directly to the sink as display output.

An application can format atomically into its own buffer if its final destination requires that behavior. The streaming API cannot undo writes already made by a failing sink or replacement `Display`.

## Plural selection

A matched text message yields a plain template. A matched plural message supplies its locale to the caller's rules, selects the corresponding template, and uses `Other` when that category is absent. A provider may capture operands of any type. The built-in provider works with plain unsigned counts for `en` and `zh` only. The English CLDR rule tests `i = 1` and `v = 0`. The `u64` input represents a plain integer with zero visible fraction digits. [UTS #35 rev. 79](https://www.unicode.org/reports/tr35/tr35-79/tr35-numbers.html#Language_Plural_Rules), [Pinned CLDR 49 cardinal data](https://github.com/unicode-org/cldr/blob/96e54b273d2da4ec6fbad350ba19ff23fa24a588/common/supplemental/plurals.xml)
