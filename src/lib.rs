extern crate proc_macro;

use model::ConvexField;
use proc_macro::TokenStream;
use syn::parse_macro_input;

mod model;

/// Create models using the same [Convex validator](https://docs.convex.dev/functions/args-validation#convex-values) syntax as your schema definition.
///
/// ```ignore
/// convex_model!(User {
///   _id: v.id("users"),
///   name: v.string(),
///   age: v.optional(v.int64()),
///   platform: v.union(
///     v.object({
///       platform: v.literal("google"),
///       verified: v.boolean(),
///     }),
///     v.object({
///       platform: v.literal("github"),
///       username: v.string(),
///     }),
///   ),
/// });
/// ```
///
/// This generates `pub struct User {}` with various methods to convert from [`convex::Value`](https://docs.rs/convex/0.6.0/convex/enum.Value.html) and to [`serde_json::Value`](https://docs.rs/serde_json/latest/serde_json/enum.Value.html).
///
/// ```ignore
/// let user = User::from_convex_value(&Value::Object(btreemap! {
///   "_id".into() => Value::String("1234".into()),
///   "name".into() => Value::String("Alice".into()),
///   "age".into() => Value::Int64(42),
///   "platform".into() => Value::Object(btreemap! {
///     "platform".into() => Value::String("github".into()),
///     "username".into() => Value::String("alicecodes".into()),
///   }),
/// }))
/// .expect("it should parse");
///
/// assert_eq!("1234", user._id);
/// assert_eq!("alicecodes", user.platform.as_2().unwrap().username);
/// assert_eq!(
///   json!({
///     "_id": "1234",
///     "name": "Alice",
///     "age": 42,
///     "platform": {
///       "platform": "github",
///       "username": "alicecodes",
///     },
///   }),
///   json!(user),
/// );
/// ```
#[proc_macro]
pub fn convex_model(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as ConvexField);
  let output = input.print();
  let ts = proc_macro2::TokenStream::from_iter(output);
  ts.into()
}
