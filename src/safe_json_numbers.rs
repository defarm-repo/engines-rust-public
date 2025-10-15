/// Safe JSON number serialization for JavaScript compatibility
///
/// JavaScript can only safely represent integers up to 2^53 - 1 (Number.MAX_SAFE_INTEGER).
/// This module provides serialization helpers to ensure numeric values are properly
/// represented in JSON responses to avoid BigInt issues in JavaScript frontends.
use serde::{Deserialize, Deserializer, Serializer};

/// Maximum safe integer in JavaScript (2^53 - 1)
pub const JS_MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// Serialize u64 as a regular JSON number if safe, or as a string if too large
pub mod u64_safe {
    use super::*;

    pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // For values within JavaScript's safe range, serialize as number
        // For larger values, serialize as string to preserve precision
        if *value <= JS_MAX_SAFE_INTEGER {
            serializer.serialize_u64(*value)
        } else {
            serializer.serialize_str(&value.to_string())
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u64, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Accept both number and string formats for flexibility
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum U64Helper {
            Number(u64),
            String(String),
        }

        match U64Helper::deserialize(deserializer)? {
            U64Helper::Number(n) => Ok(n),
            U64Helper::String(s) => s
                .parse()
                .map_err(|_| serde::de::Error::custom("invalid u64 string")),
        }
    }
}

/// Serialize i64 as a regular JSON number if safe, or as a string if too large
pub mod i64_safe {
    use super::*;

    pub fn serialize<S>(value: &i64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // JavaScript safe range for signed integers
        const JS_MIN_SAFE_INTEGER: i64 = -9_007_199_254_740_991;
        const JS_MAX_SAFE_INTEGER: i64 = 9_007_199_254_740_991;

        if *value >= JS_MIN_SAFE_INTEGER && *value <= JS_MAX_SAFE_INTEGER {
            serializer.serialize_i64(*value)
        } else {
            serializer.serialize_str(&value.to_string())
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<i64, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum I64Helper {
            Number(i64),
            String(String),
        }

        match I64Helper::deserialize(deserializer)? {
            I64Helper::Number(n) => Ok(n),
            I64Helper::String(s) => s
                .parse()
                .map_err(|_| serde::de::Error::custom("invalid i64 string")),
        }
    }
}

/// Helper for Option<u64>
pub mod option_u64_safe {
    use super::*;

    pub fn serialize<S>(value: &Option<u64>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(v) if *v <= JS_MAX_SAFE_INTEGER => serializer.serialize_some(v),
            Some(v) => serializer.serialize_some(&v.to_string()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<u64>, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum OptionU64Helper {
            None,
            Number(u64),
            String(String),
        }

        match OptionU64Helper::deserialize(deserializer)? {
            OptionU64Helper::None => Ok(None),
            OptionU64Helper::Number(n) => Ok(Some(n)),
            OptionU64Helper::String(s) => s
                .parse()
                .map(Some)
                .map_err(|_| serde::de::Error::custom("invalid u64 string")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use serde_json;

    #[derive(Serialize, Deserialize, Debug, PartialEq)]
    struct TestStruct {
        #[serde(with = "u64_safe")]
        large_value: u64,
        #[serde(with = "u64_safe")]
        safe_value: u64,
    }

    #[test]
    fn test_safe_number_serialization() {
        let test = TestStruct {
            large_value: u64::MAX,
            safe_value: 1000,
        };

        let json = serde_json::to_string(&test).unwrap();
        assert!(json.contains("\"18446744073709551615\"")); // large_value as string
        assert!(json.contains("1000")); // safe_value as number

        let deserialized: TestStruct = serde_json::from_str(&json).unwrap();
        assert_eq!(test, deserialized);
    }

    #[test]
    fn test_javascript_safe_range() {
        let test = TestStruct {
            large_value: JS_MAX_SAFE_INTEGER + 1,
            safe_value: JS_MAX_SAFE_INTEGER,
        };

        let json = serde_json::to_string(&test).unwrap();
        assert!(json.contains("\"9007199254740992\"")); // large_value as string
        assert!(json.contains("9007199254740991")); // safe_value as number
    }
}
