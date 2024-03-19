use convex::Value;
use maplit::btreemap;
use ragkit_convex_macros::convex_model;
use serde_json::json;

convex_model!(Example {
  one: v.id("table"),
  two: v.null(),
  three: v.int64(),
  four: v.number(),
  five: v.boolean(),
  six: v.string(),
  seven: v.literal("seven"),
  eight: v.literal(false),
  nine: v.literal(9),
  ten: v.union(v.string(), v.number()),
});

#[test]
fn example() {
  let convex_data = Value::Object(btreemap! {
    "one".into() => Value::String("123fakeid".into()),
    "two".into() => Value::Null,
    "three".into() => Value::Int64(42),
    "four".into() => Value::Float64(3.5),
    "five".into() => Value::Boolean(true),
    "six".into() => Value::String("Hello World".into()),
    "seven".into() => Value::String("seven".into()),
    "eight".into() => Value::Boolean(false),
    "nine".into() => Value::Int64(9),
    "ten".into() => Value::Float64(10.0),
  });

  let model =
    Example::from_convex_value(&convex_data).expect("Model should parse data");

  assert_eq!("123fakeid", model.one);
  assert_eq!(42, model.three);
  assert_eq!(3.5, model.four);
  assert!(model.five);
  assert_eq!("Hello World", model.six);
  assert_eq!("seven", model.seven);
  assert!(!model.eight);
  assert_eq!(9, model.nine);

  if let ExampleTen::Variant2(value) = model.ten {
    assert_eq!(10.0, value);
  } else {
    panic!("Expected 10")
  }

  let expected_json_data = json!({
    "one": "123fakeid",
    "two": null,
    "three": 42,
    "four": 3.5,
    "five": true,
    "six": "Hello World",
    "seven": "seven",
    "eight": false,
    "nine": 9,
    // TODO: Fix this.
    "ten": {
      "Variant2": 10.0
    },
  });

  let actual_json_data = json!(model);
  assert_eq!(expected_json_data, actual_json_data);
}
