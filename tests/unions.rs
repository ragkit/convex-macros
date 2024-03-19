use convex::Value;
use maplit::btreemap;
use ragkit_convex_macros::convex_model;
use serde_json::json;

#[test]
fn basic_discriminated() {
  convex_model!(Model {
    a: v.union(
      v.object({
        t: v.literal("one"),
        value: v.int64(),
      }),
      v.object({
        t: v.literal("two"),
        value: v.string(),
      }),
    ),
  });

  // Try variant 1
  let convex_data = Value::Object(btreemap! {
    "a".into() => Value::Object(btreemap! {
      "t".into() => Value::String("one".into()),
      "value".into() => Value::Int64(42),
    }),
  });
  let json_data = json!({
    "a": {
      "t": "one",
      "value": 42,
    },
  });

  let model = Model::from_convex_value(&convex_data);
  assert!(model.is_ok());
  let model = model.unwrap();
  let a = model.a.as_1().unwrap();
  assert_eq!("one", a.t);
  assert_eq!(42, a.value);
  assert_eq!(json_data, json!(model));

  // Try Variant 2
  let convex_data = Value::Object(btreemap! {
    "a".into() => Value::Object(btreemap! {
      "t".into() => Value::String("two".into()),
      "value".into() => Value::String("something".into()),
    }),
  });
  let json_data = json!({
    "a": {
      "t": "two",
      "value": "something",
    },
  });

  let model = Model::from_convex_value(&convex_data);
  assert!(model.is_ok());
  let model = model.unwrap();
  let a = model.a.as_2().unwrap();
  assert_eq!("two", a.t);
  assert_eq!("something", a.value);
  assert_eq!(json_data, json!(model));
}
