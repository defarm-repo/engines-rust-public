/// Standalone example demonstrating the BigInt serialization fix
/// Run with: cargo run --example test_bigint_serialization
use serde::{Deserialize, Serialize};
use serde_json;

// JavaScript's MAX_SAFE_INTEGER (2^53 - 1)
const JS_MAX_SAFE_INTEGER: u64 = 9_007_199_254_740_991;

/// Safe u64 serialization module
mod u64_safe {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(value: &u64, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
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

#[derive(Debug, Serialize, Deserialize)]
struct ApiResponse {
    #[serde(with = "u64_safe")]
    pub total_events: u64,
    #[serde(with = "u64_safe")]
    pub events_last_24h: u64,
    #[serde(with = "u64_safe")]
    pub events_last_7d: u64,
    pub description: String,
}

fn main() {
    println!("=== BigInt Serialization Fix Demo ===\n");

    // Test case 1: Safe values (within JavaScript's safe integer range)
    let response1 = ApiResponse {
        total_events: 1000,
        events_last_24h: 50000,
        events_last_7d: 350000,
        description: "Normal values - all within safe range".to_string(),
    };

    let json1 = serde_json::to_string_pretty(&response1).unwrap();
    println!("Test 1 - Safe values (serialized as numbers):");
    println!("{}\n", json1);

    // Test case 2: Mixed values (some safe, some exceed safe range)
    let response2 = ApiResponse {
        total_events: JS_MAX_SAFE_INTEGER,
        events_last_24h: JS_MAX_SAFE_INTEGER + 1,
        events_last_7d: JS_MAX_SAFE_INTEGER + 1000,
        description: "Mixed values - some exceed safe range".to_string(),
    };

    let json2 = serde_json::to_string_pretty(&response2).unwrap();
    println!("Test 2 - Mixed values:");
    println!("{}", json2);
    println!("Note: events_last_24h and events_last_7d are strings!\n");

    // Test case 3: Very large values
    let response3 = ApiResponse {
        total_events: u64::MAX,
        events_last_24h: u64::MAX - 1,
        events_last_7d: u64::MAX / 2,
        description: "Very large values - all exceed safe range".to_string(),
    };

    let json3 = serde_json::to_string_pretty(&response3).unwrap();
    println!("Test 3 - Very large values (all as strings):");
    println!("{}\n", json3);

    // Demonstrate deserialization works both ways
    println!("=== Deserialization Test ===\n");

    // JSON with numbers
    let json_with_numbers = r#"{
        "total_events": 1000,
        "events_last_24h": 2000,
        "events_last_7d": 14000,
        "description": "From JavaScript with regular numbers"
    }"#;

    let parsed1: ApiResponse = serde_json::from_str(json_with_numbers).unwrap();
    println!("Parsed from numbers: {:?}\n", parsed1);

    // JSON with strings (for large numbers from JavaScript)
    let json_with_strings = r#"{
        "total_events": "18446744073709551615",
        "events_last_24h": "9007199254740992",
        "events_last_7d": 1000,
        "description": "From JavaScript with BigInt as strings"
    }"#;

    let parsed2: ApiResponse = serde_json::from_str(json_with_strings).unwrap();
    println!("Parsed from mixed (strings and numbers): {:?}\n", parsed2);

    println!("=== Summary ===");
    println!(
        "✅ Values <= {} are serialized as JSON numbers",
        JS_MAX_SAFE_INTEGER
    );
    println!(
        "✅ Values > {} are serialized as JSON strings",
        JS_MAX_SAFE_INTEGER
    );
    println!("✅ Deserialization accepts both formats");
    println!("✅ Frontend can safely handle all values without BigInt issues");
}
