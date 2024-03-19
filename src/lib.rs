extern crate proc_macro;

use model::ConvexField;
use proc_macro::TokenStream;
use syn::parse_macro_input;

mod model;

#[proc_macro]
pub fn convex_model(input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as ConvexField);
  let output = input.print();
  let ts = proc_macro2::TokenStream::from_iter(output);
  ts.into()
}
