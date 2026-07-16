use std::sync::LazyLock;

static DEFAULT_LABEL: LazyLock<String> = LazyLock::new(|| "wire".to_owned());

pub trait Encoder {
    fn encode(&mut self, input: &[u8]) -> Vec<u8>;

    fn flush(&mut self) -> Result<(), EncodeError>;
}

#[derive(Debug, PartialEq, Eq)]
pub struct EncodeError;

pub fn default_label() -> &'static str {
    DEFAULT_LABEL.as_str()
}

pub fn label_is_initialized() -> bool {
    LazyLock::get(&DEFAULT_LABEL).is_some()
}

#[cfg(feature = "schema")]
pub fn schema_name() -> &'static str {
    schema_data::SCHEMA_NAME
}

#[cfg(feature = "schema")]
pub fn packaged_schema() -> &'static str {
    include_str!("../schema.json")
}

#[cfg(test)]
mod tests {
    #[test]
    fn exposes_default_label() {
        assert!(!super::label_is_initialized());
        assert_eq!(super::default_label(), "wire");
        assert!(super::label_is_initialized());
    }

    #[cfg(feature = "schema")]
    #[test]
    fn exposes_packaged_schema() {
        assert_eq!(super::schema_name(), "wire-v1");
        assert!(super::packaged_schema().contains("wire-v1"));
    }
}
