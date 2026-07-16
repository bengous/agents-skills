use std::sync::LazyLock;

static LABEL: LazyLock<String> = LazyLock::new(|| "fixture".to_owned());

/// Returns the label only when another caller has already initialized it.
pub fn peek_label() -> Option<&'static str> {
    LazyLock::get(&LABEL).map(String::as_str)
}

/// Initializes and returns the fixture label.
pub fn label() -> &'static str {
    LazyLock::force(&LABEL).as_str()
}

#[cfg(feature = "json")]
/// Identifies the local JSON capability.
pub fn json_capability() -> &'static str {
    json_stub::CAPABILITY
}
