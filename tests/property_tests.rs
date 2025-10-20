//! Property-based tests for DatabaseValue using proptest

use proptest::prelude::*;
use rust_database_system::prelude::*;

// ============================================================================
// DatabaseValue Roundtrip Tests
// ============================================================================

proptest! {
    /// Test that Bool values roundtrip correctly
    #[test]
    fn test_bool_roundtrip(value in any::<bool>()) {
        let db_val = DatabaseValue::from(value);
        assert_eq!(db_val.as_bool(), Some(value));
        assert!(!db_val.is_null());
        assert_eq!(db_val.type_name(), "bool");
    }

    /// Test that Int values roundtrip correctly
    #[test]
    fn test_int_roundtrip(value in any::<i32>()) {
        let db_val = DatabaseValue::from(value);
        assert_eq!(db_val.as_int(), Some(value));
        assert!(!db_val.is_null());
        assert_eq!(db_val.type_name(), "int");
    }

    /// Test that Long values roundtrip correctly
    #[test]
    fn test_long_roundtrip(value in any::<i64>()) {
        let db_val = DatabaseValue::from(value);
        assert_eq!(db_val.as_long(), Some(value));
        assert!(!db_val.is_null());
        assert_eq!(db_val.type_name(), "long");
    }

    /// Test that Float values roundtrip correctly (excluding NaN and infinities)
    #[test]
    fn test_float_roundtrip(value in any::<f32>().prop_filter("finite", |v| v.is_finite())) {
        let db_val = DatabaseValue::from(value);
        let retrieved = db_val.as_float().unwrap();
        assert!((retrieved - value).abs() < 1e-6 || retrieved == value);
        assert!(!db_val.is_null());
        assert_eq!(db_val.type_name(), "float");
    }

    /// Test that Double values roundtrip correctly (excluding NaN and infinities)
    #[test]
    fn test_double_roundtrip(value in any::<f64>().prop_filter("finite", |v| v.is_finite())) {
        let db_val = DatabaseValue::from(value);
        let retrieved = db_val.as_double().unwrap();
        assert!((retrieved - value).abs() < 1e-10 || retrieved == value);
        assert!(!db_val.is_null());
        assert_eq!(db_val.type_name(), "double");
    }

    /// Test that String values roundtrip correctly
    #[test]
    fn test_string_roundtrip(value in ".*") {
        let db_val = DatabaseValue::from(value.clone());
        assert_eq!(db_val.as_string(), value);
        assert!(!db_val.is_null());
        assert_eq!(db_val.type_name(), "string");
    }

    /// Test that Bytes values roundtrip correctly
    #[test]
    fn test_bytes_roundtrip(value in prop::collection::vec(any::<u8>(), 0..1000)) {
        let db_val = DatabaseValue::from(value.clone());
        assert_eq!(db_val.as_bytes(), Some(value.as_slice()));
        assert!(!db_val.is_null());
        assert_eq!(db_val.type_name(), "bytes");
    }
}

// ============================================================================
// Type Conversion Tests
// ============================================================================

proptest! {
    /// Test that Int to Long conversion works
    #[test]
    fn test_int_to_long_conversion(value in any::<i32>()) {
        let db_val = DatabaseValue::from(value);
        assert_eq!(db_val.as_long(), Some(value as i64));
    }

    /// Test that Int to Double conversion works
    #[test]
    fn test_int_to_double_conversion(value in any::<i32>()) {
        let db_val = DatabaseValue::from(value);
        let as_double = db_val.as_double().unwrap();
        assert!((as_double - value as f64).abs() < 1e-10);
    }

    /// Test that Float to Double conversion works
    #[test]
    fn test_float_to_double_conversion(value in any::<f32>().prop_filter("finite", |v| v.is_finite())) {
        let db_val = DatabaseValue::from(value);
        let as_double = db_val.as_double().unwrap();
        // Allow for some precision loss in conversion
        assert!((as_double - value as f64).abs() < 1e-6);
    }

    /// Test that any value can be converted to string (no panic)
    #[test]
    fn test_to_string_never_panics(value in prop_oneof![
        any::<i32>().prop_map(DatabaseValue::from),
        any::<i64>().prop_map(DatabaseValue::from),
        any::<f64>().prop_filter("finite", |v| v.is_finite()).prop_map(DatabaseValue::from),
        ".*".prop_map(DatabaseValue::from),
    ]) {
        // Should never panic
        let _ = value.as_string();
    }
}

// ============================================================================
// Null Handling Tests
// ============================================================================

proptest! {
    /// Test that Option<T>::None creates Null values
    #[test]
    fn test_null_from_none(_value in 0..100u32) {
        let db_val = DatabaseValue::from(Option::<i32>::None);
        assert!(db_val.is_null());
        assert_eq!(db_val.type_name(), "null");
        assert_eq!(db_val.as_int(), None);
        assert_eq!(db_val.as_string(), "null");
    }

    /// Test that Option<T>::Some creates non-Null values
    #[test]
    fn test_some_not_null(value in any::<i32>()) {
        let db_val = DatabaseValue::from(Some(value));
        assert!(!db_val.is_null());
        assert_eq!(db_val.as_int(), Some(value));
    }
}

// ============================================================================
// DatabaseRow Tests
// ============================================================================

proptest! {
    /// Test that DatabaseRow can store and retrieve values
    #[test]
    fn test_row_operations(
        int_val in any::<i32>(),
        string_val in ".*",
        double_val in any::<f64>().prop_filter("finite", |v| v.is_finite())
    ) {
        let mut row = DatabaseRow::new();

        row.insert("int_col".to_string(), DatabaseValue::from(int_val));
        row.insert("string_col".to_string(), DatabaseValue::from(string_val.clone()));
        row.insert("double_col".to_string(), DatabaseValue::from(double_val));

        assert_eq!(row.get("int_col").and_then(|v| v.as_int()), Some(int_val));
        assert_eq!(row.get("string_col").map(|v| v.as_string()), Some(string_val));
        assert!(row.get("double_col").and_then(|v| v.as_double()).is_some());
    }

    /// Test that DatabaseRow handles missing columns gracefully
    #[test]
    fn test_row_missing_column(column_name in "[a-z]{3,10}") {
        let row = DatabaseRow::new();
        assert!(!row.contains_key(&column_name));
    }

    /// Test that DatabaseRow can be cloned correctly
    #[test]
    fn test_row_clone(values in prop::collection::vec(any::<i32>(), 0..20)) {
        let mut row = DatabaseRow::new();

        for (i, val) in values.iter().enumerate() {
            row.insert(format!("col_{}", i), DatabaseValue::from(*val));
        }

        let cloned = row.clone();

        for (i, val) in values.iter().enumerate() {
            assert_eq!(cloned.get(&format!("col_{}", i)).and_then(|v| v.as_int()), Some(*val));
        }
    }
}

// ============================================================================
// JSON Serialization Tests
// ============================================================================

proptest! {
    /// Test that DatabaseValue JSON serialization doesn't panic
    #[test]
    fn test_json_serialization_no_panic(value in prop_oneof![
        any::<bool>().prop_map(DatabaseValue::from),
        any::<i32>().prop_map(DatabaseValue::from),
        any::<i64>().prop_map(DatabaseValue::from),
        any::<f64>().prop_filter("finite", |v| v.is_finite()).prop_map(DatabaseValue::from),
        ".*".prop_map(|s: String| DatabaseValue::from(s)),
    ]) {
        // Serialization should never panic
        let result = serde_json::to_string(&value);
        assert!(result.is_ok());
    }
}

// ============================================================================
// Timestamp Tests
// ============================================================================

// NOTE: Timestamp tests temporarily disabled pending DatabaseValue::from(DateTime)
// and as_timestamp() API implementation
//
// proptest! {
//     #[test]
//     fn test_timestamp_operations(seconds in 0i64..2_000_000_000) {
//         use chrono::{DateTime, Utc, TimeZone};
//         let dt = Utc.timestamp_opt(seconds, 0).single().unwrap();
//         let db_val = DatabaseValue::from(dt);
//         assert_eq!(db_val.type_name(), "Timestamp");
//     }
// }

// ============================================================================
// Clone and Equality Tests
// ============================================================================

proptest! {
    /// Test that DatabaseValue cloning works correctly
    #[test]
    fn test_value_clone(value in any::<i64>()) {
        let original = DatabaseValue::from(value);
        let cloned = original.clone();

        assert_eq!(original.type_name(), cloned.type_name());
        assert_eq!(original.as_long(), cloned.as_long());
        assert_eq!(original.as_string(), cloned.as_string());
        assert_eq!(original.is_null(), cloned.is_null());
    }
}

// ============================================================================
// Safety Tests (No Panics)
// ============================================================================

proptest! {
    /// Test that type conversions never panic
    #[test]
    fn test_conversions_no_panic(value in prop_oneof![
        any::<i32>().prop_map(DatabaseValue::from),
        any::<i64>().prop_map(DatabaseValue::from),
        any::<f64>().prop_filter("finite", |v| v.is_finite()).prop_map(DatabaseValue::from),
        ".*".prop_map(DatabaseValue::from),
        prop::collection::vec(any::<u8>(), 0..1000).prop_map(DatabaseValue::from),
    ]) {
        // All these conversions should be safe (return Option or have sensible defaults)
        let _ = value.as_bool();
        let _ = value.as_int();
        let _ = value.as_long();
        let _ = value.as_float();
        let _ = value.as_double();
        let _ = value.as_string();
        let _ = value.as_bytes();
        // let _ = value.as_timestamp();  // Disabled pending API implementation
        let _ = value.type_name();
        let _ = value.is_null();
    }
}
