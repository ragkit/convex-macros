# convex-macros

[![CI Badge](https://github.com/ragkit/convex-macros/actions/workflows/ci.yml/badge.svg)](https://github.com/ragkit/convex-macros/actions/workflows/ci.yml)

Macros to help make Convex in Rust nice

## Installation

```toml
[dependencies]
ragkit_convex_macros = "0.0.2"

# Required by code this macro generates.
anyhow = "1.0.80"
convex = "0.6.0"
serde = "1.0.185"
serde_json = "1.0"
```

## Usage

Create models using the same [Convex validator](https://docs.convex.dev/functions/args-validation#convex-values) syntax as your schema definition.

```rust
convex_model!(User {
  _id: v.id("users"),
  name: v.string(),
  age: v.optional(v.int64()),
  platform: v.union(
    v.object({
      platform: v.literal("google"),
      verified: v.boolean(),
    }),
    v.object({
      platform: v.literal("github"),
      username: v.string(),
    }),
  ),
});
```

This generates `pub struct User {}` with various methods to convert from [`convex::Value`](https://docs.rs/convex/0.6.0/convex/enum.Value.html) and to [`serde_json::Value`](https://docs.rs/serde_json/latest/serde_json/enum.Value.html).

```rust
let user = User::from_convex_value(&Value::Object(btreemap! {
  "_id".into() => Value::String("1234".into()),
  "name".into() => Value::String("Alice".into()),
  "age".into() => Value::Int64(42),
  "platform".into() => Value::Object(btreemap! {
    "platform".into() => Value::String("github".into()),
    "username".into() => Value::String("alicecodes".into()),
  }),
}))
.expect("it should parse");

assert_eq!("1234", user._id);
assert_eq!("alicecodes", user.platform.as_2().unwrap().username);
assert_eq!(
  json!({
    "_id": "1234",
    "name": "Alice",
    "age": 42,
    "platform": {
      "platform": "github",
      "username": "alicecodes",
    },
  }),
  json!(user),
);
```

## Features

- `let user = User::from_convex_value(value)?;` to parse a value from Convex client
- `json!(user)` to serialize as json
- Discriminated unions are automatically handled
- Helper functions for each branch are also exposed: `user.platform.as_2()?.username`

## Limitations

- This is experimental and may not be "production quality", use with caution.
- `v.bytes()`, `v.array()`, `v.any()` are not yet supported.
- Field names must be valid Rust identifiers, so keywords like `type` cannot be a field name. Map it to `_type`, `kind`, `t`, etc.
- Union variant names are always named like: `Variant1`, `Variant2`, etc.
- The first acceptable union branch will be used if there are multiples that could validly parse data.
- This package generates code that expects `anyhow`, `convex`, `serde`, and `serde_json` to be available.

# License

MIT
