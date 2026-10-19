//! URL guard: only the approved Archidekt host and a numeric deck path are
//! accepted. The request target is rebuilt from the id, never taken from the
//! pasted string.

use aperture_import::*;

#[test]
fn accepts_the_documented_deck_url_forms() {
    for input in [
        "https://archidekt.com/decks/26206384",
        "https://archidekt.com/decks/26206384/",
        "https://www.archidekt.com/decks/26206384",
        "https://www.archidekt.com/decks/26206384/",
        "https://ARCHIDEKT.COM/decks/26206384",
        "  https://archidekt.com/decks/26206384?utm_source=copy  ",
        "https://archidekt.com/decks/26206384#share",
        "https://archidekt.com/api/decks/26206384",
        "https://archidekt.com/api/decks/26206384/",
    ] {
        let parsed = parse_deck_url(input).unwrap_or_else(|e| panic!("{input} must parse: {e}"));
        assert_eq!(parsed.deck_id, 26206384, "{input}");
        assert!(parsed.url.starts_with("https://"), "{input}");
    }
}

#[test]
fn rejects_non_approved_schemes_hosts_and_paths() {
    for input in [
        "http://archidekt.com/decks/26206384",
        "ftp://archidekt.com/decks/26206384",
        "file:///C:/decks.txt",
        "javascript:alert(1)",
        "https://archidekt.com.evil.example/decks/26206384",
        "https://evil.example/decks/26206384",
        "https://archidekt.com@evil.example/decks/26206384",
        "https://user:pass@archidekt.com/decks/26206384",
        "https://archidekt.com:8443/decks/26206384",
        "https://127.0.0.1/decks/26206384",
        "https://[::1]/decks/26206384",
        "https://169.254.169.254/latest/meta-data",
        "https://archidekt.com/decks/abc",
        "https://archidekt.com/decks/",
        "https://archidekt.com/deck/26206384",
        "https://archidekt.com/decks/26206384/../../admin",
        "https://archidekt.com/decks/26206384%2f..%2fadmin",
        "https://archidekt.com/decks/26206384/extra",
        "archidekt.com/decks/26206384",
        "",
        "   ",
        "https://archidekt.com/decks/26206384\nhttps://evil.example/",
    ] {
        assert!(parse_deck_url(input).is_err(), "{input:?} must be rejected");
    }
}

#[test]
fn host_allowlist_is_exact() {
    assert!(host_allowed("archidekt.com"));
    assert!(host_allowed("ARCHIDEKT.COM"));
    assert!(host_allowed("www.archidekt.com"));
    assert!(!host_allowed("archidekt.com.evil.example"));
    assert!(!host_allowed("evilarchidekt.com"));
    assert!(!host_allowed("localhost"));
    assert!(!host_allowed("127.0.0.1"));
    assert!(!host_allowed(""));
}

#[test]
fn api_endpoint_is_built_from_the_numeric_id() {
    let parsed = parse_deck_url("https://www.archidekt.com/decks/26206384?x=1").unwrap();
    assert_eq!(
        api_url(&parsed),
        "https://archidekt.com/api/decks/26206384/"
    );
}
