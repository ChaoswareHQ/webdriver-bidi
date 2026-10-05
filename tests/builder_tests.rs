//! Builder and getter smoke tests, mirroring `browser-protocol`.

use webdriver_bidi::{browsing_context, session};

#[test]
fn activate_builder_and_defaults() {
    let params = browsing_context::ActivateParameters::builder("ctx-1").build();
    assert_eq!(
        browsing_context::ActivateParameters::METHOD,
        "browsingContext.activate"
    );
    assert_eq!(params.context(), "ctx-1");
    assert_eq!(
        serde_json::to_string(&params).unwrap(),
        r#"{"context":"ctx-1"}"#
    );
}

#[test]
fn navigate_builder_optional_setters() {
    let params = browsing_context::NavigateParameters::builder("ctx-1", "https://example.com")
        .wait(browsing_context::ReadinessState::Complete)
        .build();

    assert_eq!(params.url(), "https://example.com");
    assert_eq!(
        params.wait(),
        Some(&browsing_context::ReadinessState::Complete)
    );

    let serialized = serde_json::to_string(&params).unwrap();
    assert!(serialized.contains(r#""url":"https://example.com""#));
    assert!(serialized.contains(r#""wait":"complete""#));
}

#[test]
fn optional_fields_are_omitted_when_none() {
    let params =
        browsing_context::NavigateParameters::builder("ctx-1", "https://example.com").build();
    assert_eq!(params.wait(), None);
    assert!(!serde_json::to_string(&params).unwrap().contains("wait"));
}

#[test]
fn session_new_builder() {
    let capabilities = session::CapabilitiesRequest::builder().build();
    let params = session::NewParameters::builder(capabilities).build();
    assert_eq!(session::NewParameters::METHOD, "session.new");
    assert_eq!(
        serde_json::to_string(&params).unwrap(),
        r#"{"capabilities":{}}"#
    );
}

#[test]
fn synthesized_inline_types_are_typed() {
    // An inline `value: { ? name, ? role }` map becomes a named struct...
    let value = browsing_context::AccessibilityLocatorValue::builder()
        .name("Search")
        .build();
    let locator = browsing_context::AccessibilityLocator::builder("accessibility", value).build();
    assert_eq!(locator.value().name(), Some("Search"));

    // ...and an inline `"full" / "partial"` union becomes an enum.
    let inner = browsing_context::InnerTextLocator::builder("innerText", "hello")
        .match_type(browsing_context::InnerTextLocatorMatchType::Partial)
        .build();
    assert_eq!(inner.value(), "hello");
    assert_eq!(
        inner.match_type(),
        Some(&browsing_context::InnerTextLocatorMatchType::Partial)
    );
}
