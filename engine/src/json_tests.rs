// SPDX-License-Identifier: Apache-2.0

use super::*;

#[test]
fn roundtrip_pretty() {
    let src = r#"{"a": 1, "b": [true, null, "x\n"], "c": {"d": -2}}"#;
    let v = parse(src).unwrap();
    let pretty = v.to_json_pretty();
    let v2 = parse(&pretty).unwrap();
    assert_eq!(v, v2);
}

#[test]
fn escapes_and_unicode() {
    let v = parse(r#""aéb😀""#).unwrap();
    assert_eq!(v.as_str().unwrap(), "a\u{e9}b\u{1F600}");
    let v = parse(r#""\u0041\ud83d\ude00""#).unwrap();
    assert_eq!(v.as_str().unwrap(), "A\u{1F600}");
}

#[test]
fn integers_exact() {
    let v = parse("18446744073709551615").unwrap();
    assert_eq!(v.as_u64(), Some(u64::MAX));
    let v = parse("-5").unwrap();
    assert_eq!(v.as_u64(), None);
    // Floats are not integers for serde-typed fields.
    let v = parse("30.0").unwrap();
    assert_eq!(v.as_u64(), None);
}

#[test]
fn trailing_garbage_rejected() {
    assert!(parse("{} x").is_err());
    assert!(parse("{").is_err());
}

#[test]
fn malformed_inputs_never_panic() {
    // Every prefix of a valid doc — Err or Ok, never panic.
    let good = r#"{"a": [1, 2.5, "x", true, null, {"b": "y"}], "c": "\u00e9"}"#;
    for i in 0..=good.len() {
        if let Some(s) = good.get(..i) {
            let _ = parse(s);
        }
    }
    for bad in [
        "",
        "{",
        "[",
        "\"",
        "\\",
        "{\"a\":",
        "[1,",
        "-",
        "1e",
        "1e+",
        "0.",
        ".5",
        "+1",
        "01",
        "--1",
        "{\"a\"}",
        "[1;2]",
        "{,}",
        "[,]",
        "\"\\u\"",
        "\"\\uZZZZ\"",
        "\"\\uD800\"",
        "\"\\uD800x\"",
        "\"\\uDC00\"",
        "\"\\x\"",
        "tru",
        "truex",
        "nulll",
        "NaN",
        "Infinity",
        "{\"a\":1,}",
        "{a:1}",
        "1.2.3",
        "1e9999",
        "-0",
        "{}",
        "[]",
        "[[[]]]",
    ] {
        let _ = parse(bad);
    }
    // Deep nesting past MAX_DEPTH must Err, not overflow the stack.
    let deep = "[".repeat(1000) + &"]".repeat(1000);
    assert!(parse(&deep).is_err());
    // Control characters inside strings.
    assert!(parse("\"\u{0007}\"").is_err());
    // Lone "-" / "+" start no valid value.
    assert!(parse("[-]").is_err());
}
