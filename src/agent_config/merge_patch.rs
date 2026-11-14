//! JSON Merge Patch (RFC 7396) helpers for layering agent configuration.
//!
//! The effective config is built as: `Default` -> local `config.toml` -> server patch.

use serde_json::{Map, Value};

use crate::error::{Error, Result};
use crate::openapi::AgentConfig;

/// Apply `patch` to `target` per RFC 7396: objects merge, `null` removes a key, anything else replaces.
pub(super) fn merge_patch(target: &mut Value, patch: &Value) {
    let Value::Object(patch_map) = patch else {
        *target = patch.clone();
        return;
    };

    if !target.is_object() {
        *target = Value::Object(Map::new());
    }
    let Value::Object(target_map) = target else {
        // This should never happen because we just ensured target is an object.
        return;
    };

    for (key, value) in patch_map {
        if value.is_null() {
            target_map.remove(key);
        } else {
            merge_patch(target_map.entry(key.clone()).or_insert(Value::Null), value);
        }
    }
}

/// Start from `AgentConfig::default()` and apply `overrides`, an already merged config without nulls.
pub(super) fn layer_over_defaults(overrides: &Value) -> Result<Value> {
    let mut layered = serde_json::to_value(AgentConfig::default())?;
    merge_patch(&mut layered, overrides);
    Ok(layered)
}

/// Fail if `patch` has a key, at any depth, that `AgentConfig` does not know, or a value it cannot hold.
pub(super) fn reject_unknown_fields(patch: &Value) -> Result<()> {
    // Merging into an empty object drops the patch's nulls, as the real merge does.
    let mut applied = Value::Object(Map::new());
    merge_patch(&mut applied, patch);
    let candidate = layer_over_defaults(&applied)?;
    let config: AgentConfig = serde_json::from_value(candidate)
        .map_err(|e| Error::Config(format!("server config is not a valid agent config: {}", e)))?;
    let round_trip = serde_json::to_value(&config)?;

    match find_dropped_key(patch, &round_trip, String::new()) {
        Some(path) => Err(Error::Config(format!(
            "server config contains unknown field '{}'",
            path
        ))),
        None => Ok(()),
    }
}

/// Path of the first key present in `sent` but absent from the deserialize round trip.
fn find_dropped_key(sent: &Value, kept: &Value, path: String) -> Option<String> {
    match (sent, kept) {
        (Value::Object(sent_map), Value::Object(kept_map)) => {
            sent_map.iter().find_map(|(key, value)| {
                let child = if path.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", path, key)
                };
                match kept_map.get(key) {
                    Some(kept_value) => find_dropped_key(value, kept_value, child),
                    None => Some(child),
                }
            })
        }
        (Value::Array(sent_items), Value::Array(kept_items)) => sent_items
            .iter()
            .zip(kept_items)
            .enumerate()
            .find_map(|(i, (s, k))| find_dropped_key(s, k, format!("{}[{}]", path, i))),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    mod merge_patch_tests {
        use super::*;

        fn merged(target: Value, patch: Value) -> Value {
            let mut target = target;
            merge_patch(&mut target, &patch);
            target
        }

        // Test vectors from RFC 7396, Appendix A.
        #[test]
        fn matches_rfc_7396_appendix_a() {
            let cases = [
                (json!({"a":"b"}), json!({"a":"c"}), json!({"a":"c"})),
                (json!({"a":"b"}), json!({"b":"c"}), json!({"a":"b","b":"c"})),
                (json!({"a":"b"}), json!({"a":null}), json!({})),
                (
                    json!({"a":"b","b":"c"}),
                    json!({"a":null}),
                    json!({"b":"c"}),
                ),
                (json!({"a":["b"]}), json!({"a":"c"}), json!({"a":"c"})),
                (json!({"a":"c"}), json!({"a":["b"]}), json!({"a":["b"]})),
                (
                    json!({"a":{"b":"c"}}),
                    json!({"a":{"b":"d","c":null}}),
                    json!({"a":{"b":"d"}}),
                ),
                (json!({"a":[{"b":"c"}]}), json!({"a":[1]}), json!({"a":[1]})),
                (json!(["a", "b"]), json!(["c", "d"]), json!(["c", "d"])),
                (json!({"a":"b"}), json!(["c"]), json!(["c"])),
                (json!({"a":"foo"}), json!(null), json!(null)),
                (json!({"a":"foo"}), json!("bar"), json!("bar")),
                (json!({"e":null}), json!({"a":1}), json!({"e":null,"a":1})),
                (json!([1, 2]), json!({"a":"b","c":null}), json!({"a":"b"})),
                (
                    json!({}),
                    json!({"a":{"bb":{"ccc":null}}}),
                    json!({"a":{"bb":{}}}),
                ),
            ];

            for (target, patch, expected) in cases {
                assert_eq!(
                    merged(target.clone(), patch.clone()),
                    expected,
                    "target {target} patch {patch}"
                );
            }
        }
    }

    mod layer_over_defaults_tests {
        use super::*;

        #[test]
        fn empty_overrides_give_the_defaults() {
            let layered = layer_over_defaults(&json!({})).unwrap();

            assert_eq!(layered["monitoring"]["interval_secs"], 60);
        }

        #[test]
        fn overrides_win_over_defaults() {
            let layered =
                layer_over_defaults(&json!({"monitoring": {"interval_secs": 5}})).unwrap();

            assert_eq!(layered["monitoring"]["interval_secs"], 5);
            assert_eq!(layered["monitoring"]["timeout_secs"], 1);
        }
    }

    mod reject_unknown_fields_tests {
        use super::*;

        const ENDPOINT_ID: &str = "019680be-0000-7000-8000-000000000001";

        #[test]
        fn accepts_known_fields_and_empty_patch() {
            assert!(reject_unknown_fields(&json!({})).is_ok());
            assert!(reject_unknown_fields(&json!({
                "tags": ["a"],
                "monitoring": {"interval_secs": 30},
                "server": {"claiming": {"max_registration_retries": 2}},
                "endpoints": [{"id": ENDPOINT_ID, "address": "1.1.1.1", "enabled": true, "tags": []}]
            }))
            .is_ok());
        }

        #[test]
        fn accepts_null_to_reset_a_field() {
            assert!(reject_unknown_fields(&json!({"monitoring": {"interval_secs": null}})).is_ok());
        }

        #[test]
        fn rejects_unknown_top_level_and_nested_keys() {
            let top = reject_unknown_fields(&json!({"bogus": 1})).unwrap_err();
            let nested = reject_unknown_fields(&json!({"monitoring": {"abc": 1}})).unwrap_err();

            assert!(top.to_string().contains("bogus"), "{top}");
            assert!(nested.to_string().contains("monitoring.abc"), "{nested}");
        }

        #[test]
        fn rejects_unknown_key_even_when_its_value_is_null() {
            assert!(reject_unknown_fields(&json!({"bogus": null})).is_err());
        }

        #[test]
        fn rejects_unknown_key_inside_an_endpoint() {
            let err = reject_unknown_fields(&json!({
                "endpoints": [{"id": ENDPOINT_ID, "address": "x", "enabled": true, "tags": [], "typo": 1}]
            }))
            .unwrap_err();

            assert!(err.to_string().contains("endpoints[0].typo"), "{err}");
        }

        #[test]
        fn rejects_wrong_type_and_incomplete_endpoint() {
            assert!(
                reject_unknown_fields(&json!({"monitoring": {"interval_secs": "fast"}})).is_err()
            );
            assert!(
                reject_unknown_fields(&json!({"endpoints": [{"address": "8.8.8.8"}]})).is_err()
            );
        }
    }
}
