use convex::Value;
use maplit::btreemap;
use ragkit_convex_macros::convex_model;
use serde_json::json;

#[test]
fn basic_string() {
  convex_model!(Model { a: v.string() });
  let convex_data = Value::Object(btreemap! {
    "a".into() => Value::String("apple".into()),
  });
  let json_data = json!({
    "a": "apple",
  });

  let model = Model::from_convex_value(&convex_data);
  assert!(model.is_ok());
  let model = model.unwrap();
  assert_eq!("apple", model.a);
  assert_eq!(json_data, json!(model));
}

#[test]
fn basic_string_negative() {
  convex_model!(Model { a: v.string() });

  let model = Model::from_convex_value(&Value::Object(btreemap! {
    "a".into() => Value::Int64(42),
  }));
  assert!(model.is_err());

  let model = Model::from_convex_value(&Value::Object(btreemap! {
    "b".into() => Value::String("apple".into()),
  }));
  assert!(model.is_err());
}

#[test]
fn basic_id() {
  convex_model!(Model { a: v.id("apples") });
  let convex_data = Value::Object(btreemap! {
    "a".into() => Value::String("1234".into()),
  });
  let json_data = json!({
    "a": "1234",
  });

  let model = Model::from_convex_value(&convex_data);
  assert!(model.is_ok());
  let model = model.unwrap();
  assert_eq!("1234", model.a);
  assert_eq!(json_data, json!(model));
}

#[test]
fn basic_id_negative() {
  convex_model!(Model { a: v.id("apples") });

  let model = Model::from_convex_value(&Value::Object(btreemap! {
    "a".into() => Value::Int64(42),
  }));
  assert!(model.is_err());

  let model = Model::from_convex_value(&Value::Object(btreemap! {
    "b".into() => Value::String("apple".into()),
  }));
  assert!(model.is_err());
}

#[test]
fn basic_string_literal() {
  convex_model!(Model { a: v.literal("apples") });
  let convex_data = Value::Object(btreemap! {
    "a".into() => Value::String("apples".into()),
  });
  let json_data = json!({
    "a": "apples",
  });

  let model = Model::from_convex_value(&convex_data);
  assert!(model.is_ok());
  let model = model.unwrap();
  assert_eq!("apples", model.a);
  assert_eq!(json_data, json!(model));
}

#[test]
fn basic_string_literal_negative() {
  convex_model!(Model { a: v.literal("apples") });

  let model = Model::from_convex_value(&Value::Object(btreemap! {
    "a".into() => Value::String("not-apple".into()),
  }));
  assert!(model.is_err());

  let model = Model::from_convex_value(&Value::Object(btreemap! {
    "a".into() => Value::Int64(42),
  }));
  assert!(model.is_err());

  let model = Model::from_convex_value(&Value::Object(btreemap! {
    "b".into() => Value::String("apple".into()),
  }));
  assert!(model.is_err());
}

#[test]
fn basic_optional_string() {
  convex_model!(Model { a: v.optional(v.string()) });

  let convex_data = Value::Object(btreemap! {
    "a".into() => Value::String("apples".into()),
  });
  let json_data = json!({
    "a": "apples",
  });

  let model = Model::from_convex_value(&convex_data);
  assert!(model.is_ok());
  let model = model.unwrap();
  assert_eq!(Some("apples".into()), model.a);
  assert_eq!(json_data, json!(model));

  let convex_data = Value::Object(btreemap! {
    "a".into() => Value::Null,
  });
  let json_data = json!({
    "a": null,
  });

  let model = Model::from_convex_value(&convex_data);
  assert!(model.is_ok());
  let model = model.unwrap();
  assert_eq!(None, model.a);
  assert_eq!(json_data, json!(model));
}

#[test]
fn basic_string_union() {
  convex_model!(Model { a: v.union(v.literal("apples"), v.literal("banana")) });

  let convex_data = Value::Object(btreemap! {
    "a".into() => Value::String("apples".into()),
  });
  let json_data = json!({
    "a": "apples",
  });

  let model = Model::from_convex_value(&convex_data);
  assert!(model.is_ok());
  let model = model.unwrap();
  assert_eq!(ModelA::Variant1("apples".into()), model.a);
  assert_eq!(json_data, json!(model));

  let convex_data = Value::Object(btreemap! {
    "a".into() => Value::String("banana".into()),
  });
  let json_data = json!({
    "a": "banana",
  });

  let model = Model::from_convex_value(&convex_data);
  assert!(model.is_ok());
  let model = model.unwrap();
  assert_eq!(ModelA::Variant2("banana".into()), model.a);
  assert_eq!(json_data, json!(model));
}
