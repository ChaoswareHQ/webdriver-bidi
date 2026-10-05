//! Round-trip checks for the generated BiDi protocol types.

use bidi_protocol::{browsing_context, session, Command};

#[test]
fn builds_and_serializes_a_command_envelope() {
    let params = browsing_context::NavigateParameters::builder("ctx-1", "https://example.com")
        .wait(browsing_context::ReadinessState::Complete)
        .build();
    let command = Command::new(7, &params);
    let json = serde_json::to_value(&command).unwrap();

    assert_eq!(json["id"], 7);
    assert_eq!(json["method"], "browsingContext.navigate");
    assert_eq!(json["params"]["url"], "https://example.com");
    assert_eq!(json["params"]["wait"], "complete");
    assert_eq!(
        browsing_context::NavigateParameters::METHOD,
        "browsingContext.navigate"
    );

    // Generated getters borrow without cloning.
    assert_eq!(params.url(), "https://example.com");
    assert_eq!(
        params.wait(),
        Some(&browsing_context::ReadinessState::Complete)
    );
}

#[test]
fn deserializes_a_session_new_result() {
    let raw = r#"{
        "id": 1,
        "type": "success",
        "result": {
            "sessionId": "s1",
            "capabilities": {
                "acceptInsecureCerts": false,
                "browserName": "firefox",
                "browserVersion": "128.0",
                "platformName": "windows",
                "setWindowRect": true,
                "userAgent": "Mozilla/5.0"
            }
        }
    }"#;
    let reply: bidi_protocol::SuccessResponse<session::NewResult> =
        serde_json::from_str(raw).unwrap();
    assert_eq!(reply.result.session_id, "s1");
    assert_eq!(reply.result.capabilities.browser_name, "firefox");
}

#[test]
fn carries_command_and_event_metadata() {
    assert_eq!(session::NewParameters::METHOD, "session.new");
    assert_eq!(session::SubscribeParameters::METHOD, "session.subscribe");
    assert_eq!(
        browsing_context::NavigateParameters::METHOD,
        "browsingContext.navigate"
    );
    assert_eq!(browsing_context::Load::METHOD, "browsingContext.load");
    assert_eq!(
        browsing_context::ContextCreated::METHOD,
        "browsingContext.contextCreated"
    );
}

#[test]
fn flattens_event_params() {
    let raw = r#"{
        "children": null,
        "clientWindow": "cw-1",
        "context": "ctx-1",
        "originalOpener": null,
        "url": "https://example.com",
        "userContext": "default",
        "hasPlannedNavigation": false
    }"#;
    let event: browsing_context::ContextCreated = serde_json::from_str(raw).unwrap();
    assert_eq!(event.params.base_info.context, "ctx-1");
    assert!(!event.params.has_planned_navigation);
}
