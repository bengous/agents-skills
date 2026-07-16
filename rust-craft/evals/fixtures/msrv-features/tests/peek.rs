use msrv_features_fixture::peek_label;

#[test]
fn peek_does_not_initialize_the_label() {
    assert_eq!(peek_label(), None);
}

#[cfg(feature = "json")]
#[test]
fn json_feature_exposes_the_local_capability() {
    assert_eq!(msrv_features_fixture::json_capability(), "json");
}
