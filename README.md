# webdriver-bidi

Generated Rust types for the [W3C WebDriver BiDi](https://w3c.github.io/webdriver-bidi/)
protocol — the backend that lets Xcelerate drive **Firefox** (and every other
browser that implements BiDi) the way `browser-protocol` / `js-protocol` let it
drive Chromium over CDP.

The `webdriver-bidi-macros` companion crate provides the `BidiBuilder`,
`BidiCommand` and `BidiEvent` derives.

## Why this crate exists

Xcelerate's core speaks CDP. Firefox does **not** speak CDP (Mozilla removed it:
`remote.active-protocols` was dropped in Firefox 141 and WebDriver BiDi is the
only remaining remote protocol). BiDi's command envelope is `{id, method, params}`
— the same shape as CDP — so a sibling typed protocol crate keeps the Firefox
backend as idiomatic as the Chromium one.

## How it is harvested (the important part)

`browser-protocol` and `js-protocol` are generated from two machine-readable JSON
files that Google publishes:

* `https://raw.githubusercontent.com/ChromeDevTools/devtools-protocol/master/json/browser_protocol.json`
* `.../js_protocol.json`

WebDriver BiDi has **no equivalent JSON**. Its definitions live as **CDDL**
embedded in the Bikeshed specification source. Both the W3C and Google consume it
by extracting the CDDL first:

* W3C: `w3c/webdriver-bidi` → `scripts/cddl/generate.js` extracts every
  `<pre class="cddl">` block from `index.bs` into `local.cddl` / `remote.cddl` /
  `all.cddl`.
* Google: `GoogleChromeLabs/webdriver-bidi-protocol` pins the spec as a submodule
  and runs `cddlconv all.cddl > src/gen/main.ts`.

This crate mirrors that pipeline in one script:

```
index.bs  ──extract <pre class="cddl">──▶  all.cddl  ──parse CDDL──▶  Rust
```

```bash
python scripts/generate_rust_code.py            # download the spec + regenerate
python scripts/generate_rust_code.py --spec path/to/index.bs
python scripts/generate_rust_code.py --check    # exit 1 when the spec changed
```

`webdriver_bidi.bs` and `all.cddl` are cached in the crate root; `src/` is
generated and should not be edited by hand.

## Layout

| BiDi module (CDDL) | Rust module |
|---|---|
| `session` | `session` |
| `browsingContext` | `browsing_context` |
| `script` | `script` |
| `input` | `input` |
| `network` | `network` |
| `browser` | `browser` |
| `emulation` | `emulation` |
| `storage` | `storage` |
| `log` | `log` |
| `webExtension` | `web_extension` |

The wire envelope (`Command`, `SuccessResponse`, `ErrorResponse`, `EmptyResult`,
`EmptyParams`, and the `BidiCommand` / `BidiEvent` traits) is hand-written in
`src/lib.rs`; everything else is generated.

## Usage

```rust
use webdriver_bidi::{Command, browsing_context};

let params = browsing_context::NavigateParameters::builder("abc", "https://example.com")
    .wait(browsing_context::ReadinessState::Complete)
    .build();
let command = Command::new(1, &params);
// browsing_context::NavigateParameters::METHOD == "browsingContext.navigate"
```

## Publishing

Standalone workspace (its own, not a member of the Xcelerate workspace). Both
crates are published to crates.io independently, in dependency order:

```bash
python scripts/publish.py            # webdriver-bidi-macros, then webdriver-bidi
python scripts/publish.py --dry-run
```

## Notes

* Nullable unions (`X / null`) become `Option<X>`.
* Inline anonymous maps and string-literal unions inside a field are emitted as
  named, synthesized types (e.g. `AccessibilityLocatorValue`,
  `InnerTextLocatorMatchType`).
* Discriminated unions (`Locator`, `ClipRectangle`, `ProxyConfiguration`) are
  `#[serde(untagged)]` enums resolved by shape.
* Recursive types are boxed to keep them sized.
* `js-uint` maps to `u64` and `text` to `String` (owned, not `Cow`).
