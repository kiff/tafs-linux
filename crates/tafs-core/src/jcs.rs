//! RFC 8785 (JSON Canonicalization Scheme), as planque and KIFF compute a digest (the iPad's
//! `JCS`): keys sort by UTF-16 code units, numbers print as ECMAScript does, and an integer beyond
//! 2^53 - 1 is refused, as planque and KIFF refuse it.
//!
//! It works on `serde_json::Value`. A number written with a fraction or an exponent is read as a
//! double, which ECMAScript prints from its shortest round-tripping digits, so it comes out as the
//! iPad's does from the literal. An integer that overflows 64 bits is read as a double by serde
//! and printed as one, where the iPad refuses it: such a profile's digest then differs from
//! KIFF's, and its words are refused as another profile's, never shown.

use serde_json::Value;
use sha2::{Digest, Sha256};

/// Why a value has no canonical form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Failure {
    UnsafeInteger,
    NotFinite,
}

/// 2^53 - 1.
pub const MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// The canonical form of a value.
pub fn canonical(v: &Value) -> Result<String, Failure> {
    let mut out = String::new();
    write(v, &mut out)?;
    Ok(out)
}

/// The hex SHA-256 of a value's canonical form (`ProfileWords.digest`).
pub fn digest(v: &Value) -> Result<String, Failure> {
    let c = canonical(v)?;
    Ok(Sha256::digest(c.as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

fn write(v: &Value, out: &mut String) -> Result<(), Failure> {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Value::Number(n) => out.push_str(&number(n)?),
        Value::String(s) => string(s, out),
        Value::Array(a) => {
            out.push('[');
            for (i, e) in a.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write(e, out)?;
            }
            out.push(']');
        }
        Value::Object(o) => {
            let mut keys: Vec<&String> = o.keys().collect();
            keys.sort_by(|a, b| a.encode_utf16().cmp(b.encode_utf16()));
            out.push('{');
            for (i, k) in keys.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                string(k, out);
                out.push(':');
                write(&o[k], out)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

fn string(s: &str, out: &mut String) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{0C}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn number(n: &serde_json::Number) -> Result<String, Failure> {
    if let Some(u) = n.as_u64() {
        if u > MAX_SAFE_INTEGER {
            return Err(Failure::UnsafeInteger);
        }
        return Ok(u.to_string());
    }
    if let Some(i) = n.as_i64() {
        if i.unsigned_abs() > MAX_SAFE_INTEGER {
            return Err(Failure::UnsafeInteger);
        }
        return Ok(i.to_string());
    }
    ecma_script(n.as_f64().ok_or(Failure::NotFinite)?)
}

/// ECMAScript's Number::toString, which RFC 8785 uses for every number. Rust's `{:e}` already
/// gives the shortest digits that round-trip; this only places them.
pub fn ecma_script(d: f64) -> Result<String, Failure> {
    if !d.is_finite() {
        return Err(Failure::NotFinite);
    }
    if d == 0.0 {
        return Ok("0".into());
    }
    if d < 0.0 {
        return Ok(format!("-{}", ecma_script(-d)?));
    }
    let sci = format!("{d:e}");
    let (mantissa, exp) = sci.split_once('e').expect("{:e} has an exponent");
    let exp: i32 = exp.parse().expect("an exponent");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let s = digits.trim_end_matches('0');
    let s = if s.is_empty() { "0" } else { s };
    let k = s.len() as i32;
    // The decimal point sits after the first digit, so n = exp + 1.
    let n = exp + 1;
    Ok(if k <= n && n <= 21 {
        format!("{s}{}", "0".repeat((n - k) as usize))
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &s[..n as usize], &s[n as usize..])
    } else if -6 < n && n <= 0 {
        format!("0.{}{s}", "0".repeat((-n) as usize))
    } else {
        let e = n - 1;
        let sign = if e < 0 { '-' } else { '+' };
        let head = if k == 1 {
            s.to_string()
        } else {
            format!("{}.{}", &s[..1], &s[1..])
        };
        format!("{head}e{sign}{}", e.abs())
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn canon(input: &str) -> Result<String, Failure> {
        canonical(&serde_json::from_str(input).unwrap())
    }

    /// planque's jcs tests (RFC 8785 §3.2.2, UTF-16 key order, nesting), as the iPad's.
    #[test]
    fn canonical_is_planques() {
        let cases = [
            (
                r#"{"numbers":[333333333.33333329,1E30,4.50,2e-3,0.000000000000000000000000001],"string":"\u20ac$\u000F\u000aA'\u0042\u0022\u005c\\\"\/","literals":[null,true,false]}"#,
                r#"{"literals":[null,true,false],"numbers":[333333333.3333333,1e+30,4.5,0.002,1e-27],"string":"€$\u000f\nA'B\"\\\\\"/"}"#,
            ),
            (
                r#"{"\ud83d\ude00":1,"\ufb33":2,"a":3}"#,
                r#"{"a":3,"😀":1,"דּ":2}"#,
            ),
            (
                r#"{"b":{"z":1,"a":[1,2]},"a":"x"}"#,
                r#"{"a":"x","b":{"a":[1,2],"z":1}}"#,
            ),
        ];
        for (input, want) in cases {
            assert_eq!(canon(input).unwrap(), want, "{input}");
        }
    }

    #[test]
    fn numbers_print_as_ecmascript() {
        let cases = [
            (0.0, "0"),
            (1.0, "1"),
            (-1.0, "-1"),
            (100.0, "100"),
            (1e21, "1e+21"),
            (1e20, "100000000000000000000"),
            (0.000001, "0.000001"),
            (0.0000001, "1e-7"),
            (123.456, "123.456"),
            (9007199254740991.0, "9007199254740991"),
            (5e-324, "5e-324"),
            (f64::MAX, "1.7976931348623157e+308"),
            (-0.0, "0"),
            (1.5, "1.5"),
        ];
        for (d, want) in cases {
            assert_eq!(ecma_script(d).unwrap(), want, "{d}");
        }
        assert_eq!(ecma_script(f64::NAN), Err(Failure::NotFinite));
    }

    /// The same refusals as planque's and KIFF's.
    #[test]
    fn unsafe_integers_are_refused() {
        for input in [
            r#"{"id":9007199254740992}"#,
            r#"{"id":9007199254740993}"#,
            r#"{"a":[{"id":-9007199254740993}]}"#,
        ] {
            assert_eq!(canon(input), Err(Failure::UnsafeInteger), "{input}");
        }
        assert!(canon(r#"{"id":9007199254740991,"f":1e300}"#).is_ok());
    }

    /// A grant request, and its digest as planque computes it (the iPad's `testDigestIsPlanques`).
    #[test]
    fn digest_is_planques() {
        let request = r#"{"v":"planque.grant.v1","grant":"g_01","tenant":"t_01","server":"https://mcp.stripe.com","sign_in_at":"https://access.stripe.com","agent":"refunds","tools":[{"tool":"create_refund","name":"refund_payment","amount_argument":"amount","amount_unit":"EUR","amount_scale":2,"fixed_arguments":{"reason":"requested_by_customer","metadata.n":1.50,"big":1e30}},{"tool":"list_payments","read_only":true}],"asked_by":"setup-agent","created_at":"2026-10-17T10:00:00Z","note":"é😀\u000f"}"#;
        let v: Value = serde_json::from_str(request).unwrap();
        assert_eq!(
            canonical(&v).unwrap(),
            r#"{"agent":"refunds","asked_by":"setup-agent","created_at":"2026-10-17T10:00:00Z","grant":"g_01","note":"é😀\u000f","server":"https://mcp.stripe.com","sign_in_at":"https://access.stripe.com","tenant":"t_01","tools":[{"amount_argument":"amount","amount_scale":2,"amount_unit":"EUR","fixed_arguments":{"big":1e+30,"metadata.n":1.5,"reason":"requested_by_customer"},"name":"refund_payment","tool":"create_refund"},{"read_only":true,"tool":"list_payments"}],"v":"planque.grant.v1"}"#
        );
        assert_eq!(
            digest(&v).unwrap(),
            "6bc63ba1c1235801b7ddcb66a2e66947faefdd819e0ac0bb669062a063c048a3"
        );
    }
}
