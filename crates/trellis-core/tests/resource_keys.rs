use trellis_core::ResourceKey;

#[test]
fn structured_resource_key_preserves_segments_without_flat_parsing() {
    let key =
        ResourceKey::from_segments(["article-feed", "acct/with/slash", "home/main", "relay/a"]);

    assert_eq!(key.segment_count(), 4);
    assert_eq!(
        key.segments().collect::<Vec<_>>(),
        vec!["article-feed", "acct/with/slash", "home/main", "relay/a"]
    );
    assert_eq!(key.segment(2), Some("home/main"));
    assert_ne!(
        key.as_str(),
        "article-feed/acct/with/slash/home/main/relay/a"
    );
}

#[test]
fn single_segment_resource_key_keeps_legacy_string_view() {
    let key = ResourceKey::new("resource:1");

    assert_eq!(key.as_str(), "resource:1");
    assert_eq!(key.segments().collect::<Vec<_>>(), vec!["resource:1"]);
}

#[test]
fn encoded_resource_key_view_disambiguates_reserved_single_segments() {
    let structured = ResourceKey::from_segments(["a", "bc"]);
    let single = ResourceKey::new(structured.as_str());
    let escaped_single = ResourceKey::new("segment:1:a");

    assert_ne!(single, structured);
    assert_ne!(single.as_str(), structured.as_str());
    assert_eq!(
        single.segments().collect::<Vec<_>>(),
        vec!["segments:2:1:a2:bc"]
    );
    assert_ne!(escaped_single.as_str(), "segment:1:a");
    assert_eq!(escaped_single.segment(0), Some("segment:1:a"));
}

#[test]
fn wildcard_resource_key_is_explicit_structured_identity() {
    let key = ResourceKey::wildcard("all-devices");

    assert_eq!(
        key.segments().collect::<Vec<_>>(),
        vec!["wildcard", "all-devices"]
    );
}
