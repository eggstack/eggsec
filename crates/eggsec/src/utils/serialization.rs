use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::{Result, EggsecError};

pub fn serialize_to_json<T: Serialize>(value: &T) -> Result<String> {
    serde_json::to_string(value).map_err(|e| {
        EggsecError::Parse(format!("JSON serialization failed: {}", e))
    })
}

pub fn deserialize_from_json<T: DeserializeOwned>(json: &str) -> Result<T> {
    serde_json::from_str(json).map_err(|e| {
        EggsecError::Parse(format!("JSON deserialization failed: {}", e))
    })
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use serde::Serialize;

    use super::*;

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestStruct {
        name: String,
        value: i32,
    }

    #[test]
    fn test_serialize_deserialize_roundtrip() {
        let original = TestStruct {
            name: "test".to_string(),
            value: 42,
        };

        let json = serialize_to_json(&original).unwrap();
        let deserialized: TestStruct = deserialize_from_json(&json).unwrap();

        assert_eq!(original, deserialized);
    }

    #[test]
    fn test_deserialize_invalid_json() {
        let result: Result<TestStruct> = deserialize_from_json("not valid json");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("JSON deserialization failed"));
    }
}
