# Self-review

The documentation starts with the application problem and a working example. It then states the feature, locale, lookup, rendering, and plural contracts. The design notes explain what each operation validates and what data it borrows.

## Claims and checks

| Claim | Evidence |
| --- | --- |
| Locale structure is checked separately from registry validity | `accepts_structural_language_tags_and_preserves_spelling`, `rejects_malformed_language_tags`, and duplicate variant and singleton checks in `tests/locale_catalog.rs` |
| Fallback is per key, ordered, and observable. Requested parents handle repeated private-use singletons and grandfathered tags | `looks_up_each_key_through_exact_parent_and_ordered_fallback_catalogs`, `truncates_extensions_private_use_and_grandfathered_tags`, and `lookup_crosses_extension_and_private_use_singletons` |
| Configured fallbacks require exact catalog membership. An empty list uses only the requested hierarchy | `rejects_configured_fallbacks_without_an_exact_catalog` and `configured_fallback_checks_exact_catalog_only` |
| Catalog validation catches duplicate identities and malformed templates | Duplicate locale and key checks and invalid text and plural template checks in `tests/locale_catalog.rs` and `tests/plurals.rs` |
| Formatting accepts Unicode and opaque replacement values and makes zero writes on input errors | `unicode_escapes_repeated_placeholders_and_opaque_replacements`, `malformed_placeholders_return_utf8_byte_offsets`, and `preflight_failures_write_nothing_and_do_not_call_display` |
| Valid extra arguments are ignored and failed output may leave a prefix | `valid_extra_arguments_are_ignored_and_names_are_case_sensitive` and the failing sink and replacement tests in `tests/formatting.rs` |
| Plural selection uses the actual catalog locale and supports the full unsigned integer range | `plural_provider_uses_the_actual_fallback_catalog`, `selects_caller_categories_and_uses_other_for_absent_forms`, and `built_in_cardinal_rules_preserve_u64_bounds_and_reject_other_languages` |
| Default and alloc-only configurations build without the standard library | Library checks for both features on `wasm32-unknown-unknown` and `x86_64-unknown-none` |
| The package can be used independently of its Git checkout | `scripts/verify_package.py` checks archive bytes against Git, inspects Cargo resolution, and runs core, alloc, and std consumers outside the repository |
| The application adapter preserves the existing English version line | `preserves_english_version_line` compares exact bytes, including the newline |

## Five review dimensions

| Dimension | Review |
| --- | --- |
| Contribution | Project Z gets a small shared catalog and formatting API. The contract makes a fallback observable and leaves application language policy with the caller. |
| Writing clarity | The executable README example shows both languages and an actual fallback. Locale, formatted text, and plural limits appear where callers choose those APIs. |
| Experimental strength | Assertions exercise positive and negative behavior, including no writes on validation failure. The documentation makes no comparative throughput or resource claims. |
| Evaluation completeness | Local coverage includes the native feature matrix, Rust 1.85.0, two no-std cross-target checks, and the extracted package. Native macOS and Windows confirmation belongs to the formal workflow. |
| Method design soundness | The same grammar validates catalog entries and standalone templates. Tests check output failures separately from preflight, and verify that plural rules receive the matched locale. |

Review dimensions adapted from the [paper-review guide](https://github.com/Master-cai/Research-Paper-Writing-Skills/blob/main/research-paper-writing/references/paper-review.md).
