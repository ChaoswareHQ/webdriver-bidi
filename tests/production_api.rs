//! Smoke tests for the patterns a production BiDi client relies on:
//! generic command encoding, typed decoding, typed events, unions and errors.

use serde::de::DeserializeOwned;
use webdriver_bidi::{
    browsing_context, BidiCommand, BidiEvent, Command, EmptyResult, Reply, SuccessResponse,
};

/// Generic request encoder: `BidiCommand` provides the wire method, `Serialize` the body.
fn encode<T: BidiCommand>(id: u64, params: &T) -> String {
    serde_json::to_string(&Command::new(id, params)).unwrap()
}

/// Generic response decoder: correlates by id and yields the typed `SuccessResponse`.
fn decode<T: BidiCommand>(json: &str) -> SuccessResponse<T::Response>
where
    T::Response: DeserializeOwned,
{
    serde_json::from_str(json).unwrap()
}

#[test]
fn command_encodes_to_wire_format() {
    let params =
        browsing_context::NavigateParameters::builder("ctx-1", "https://example.com").build();
    assert_eq!(
        encode(7, &params),
        r#"{"id":7,"method":"browsingContext.navigate","params":{"context":"ctx-1","url":"https://example.com"}}"#
    );
}

#[test]
fn response_decodes_to_typed_result() {
    let response = decode::<browsing_context::NavigateParameters>(
        r#"{"id":7,"result":{"navigation":null,"url":"https://example.com/"}}"#,
    );
    assert_eq!(response.id, 7);
    assert_eq!(response.result.url, "https://example.com/");
    assert_eq!(response.result.navigation, None);
}

#[test]
fn events_are_typed_and_carry_method() {
    assert_eq!(browsing_context::Load::METHOD, "browsingContext.load");

    // The `BidiEvent` trait makes events usable generically.
    fn method_of<E: BidiEvent>() -> &'static str {
        E::METHOD
    }
    assert_eq!(
        method_of::<browsing_context::Load>(),
        "browsingContext.load"
    );
    assert_eq!(
        method_of::<browsing_context::ContextCreated>(),
        "browsingContext.contextCreated"
    );
}

#[test]
fn discriminated_union_decodes_by_shape() {
    let locator: browsing_context::Locator =
        serde_json::from_str(r##"{"type":"css","value":"#main"}"##).unwrap();
    match locator {
        browsing_context::Locator::CssLocator(css) => assert_eq!(css.value, "#main"),
        other => panic!("expected CssLocator, got {other:?}"),
    }
}

#[test]
fn error_reply_is_detected() {
    let reply: Reply<EmptyResult> = serde_json::from_str(
        r#"{"id":1,"type":"error","error":"invalid argument","message":"bad"}"#,
    )
    .unwrap();
    match reply {
        Reply::Err(error) => {
            assert_eq!(error.error, "invalid argument");
            assert_eq!(error.message, "bad");
        }
        Reply::Ok(_) => panic!("expected an error reply"),
    }
}
