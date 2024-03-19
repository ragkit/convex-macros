use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::parse::{Parse, ParseBuffer, ParseStream};
use syn::{Error, Ident, Lit, Result, Token};

#[derive(Clone)]
pub struct ConvexName {
  pub path: Vec<String>,
  pub id: String,
}

#[derive(Clone)]
pub struct ConvexField {
  pub name: ConvexName,
  pub t: ConvexType,
}

// See: https://docs.convex.dev/functions/args-validation
#[derive(Clone)]
pub enum ConvexType {
  // Core types.
  Id(String),
  Null,
  Int64,
  Number,
  Bool,
  String,
  // TODO: Bytes,
  // TODO: Array(ConvexType),
  Object(Vec<ConvexField>),
  Union(Vec<ConvexType>),
  StringLiteral(String),
  BoolLiteral(bool),
  IntLiteral(i64),
  // TODO: Any,
  Optional(Box<ConvexField>),
}

impl ConvexType {
  fn print(&self) -> Option<TokenStream> {
    match &self {
      // TODO: Improve Id types.
      | ConvexType::Id(_) => Some(quote! { String }),
      | ConvexType::Null => Some(quote! { () }),
      | ConvexType::Int64 => Some(quote! { i64 }),
      | ConvexType::Number => Some(quote! { f64 }),
      | ConvexType::Bool => Some(quote! { bool }),
      | ConvexType::String => Some(quote! { String }),

      // TODO: Can rust represent literal types?
      | ConvexType::StringLiteral(_) => Some(quote! { String }),
      | ConvexType::BoolLiteral(_) => Some(quote! { bool }),
      | ConvexType::IntLiteral(_) => Some(quote! { i64 }),

      // Kinda a weird one, we technically know the full type even if the child
      // is an Object or Union, but other parts of the system rely on returning
      // None here to communicate the Optional wraps a "complex" type.
      | ConvexType::Optional(child) => {
        child.t.print().map(|ts| quote! { Option<#ts> })
      },

      // These depend on field.name to generate a struct name.
      | ConvexType::Object(_) => None,
      | ConvexType::Union(_) => None,
    }
  }
}

impl ConvexName {
  fn to_struct_name(&self) -> Ident {
    let path_parts: Vec<String> =
      self.path.iter().map(|p| capitalize_first_char(p)).collect();
    let id_part = capitalize_first_char(&self.id);
    let s = [path_parts.join(""), id_part].join("");
    Ident::new(s.as_str(), Span::call_site())
  }

  fn to_field_name(&self) -> Ident {
    Ident::new(self.id.as_str(), Span::call_site())
  }

  fn full_path(&self) -> Vec<String> {
    let mut v = self.path.clone();
    v.push(self.id.clone());
    v
  }
}

impl Parse for ConvexField {
  fn parse(input: ParseStream) -> Result<Self> {
    let ident = Ident::parse(input)?;
    let name = ConvexName { path: Vec::new(), id: ident.to_string() };

    let content;
    let _ = syn::braced!(content in input);
    let ts = Self::parse_comma_separated(&name, &content, |name, b| {
      Self::parse_child(name, b)
    })?;

    Ok(Self { name, t: ConvexType::Object(ts) })
  }
}

impl ConvexField {
  pub fn print(&self) -> Vec<TokenStream> {
    let struct_name = self.name.to_struct_name();
    let struct_name_str = struct_name.to_string();
    let mut structs = Vec::new();
    let mut impls = Vec::new();
    let ignore_attributes = quote! {
      #[allow(non_snake_case)]
    };
    // Note: We need a custom serialize to avoid unions printing as objects.
    let enum_struct_attributes = quote! {
      #[derive(::serde::Deserialize, Clone, Debug, PartialEq)]
    };

    match &self.t {
      | ConvexType::Object(fields) => {
        structs.append(&mut Self::print_structs(fields, &struct_name));
        impls.push(Self::print_to_json_impl(fields, &struct_name));
        impls.push(Self::print_from_convex_value(fields, &struct_name));
      },
      | ConvexType::Union(types) => {
        let field_name = self.name.to_field_name();
        let field_name_str = field_name.to_string();
        let mut enum_kinds = Vec::new();
        let mut extract_arms = Vec::new();
        let mut json_arms: Vec<TokenStream> = Vec::new();
        let mut serialize_arms: Vec<TokenStream> = Vec::new();
        let mut as_fns: Vec<TokenStream> = Vec::new();
        let mut i = 0;
        for t in types {
          i += 1;
          let branch_name =
            Ident::new(format!("Variant{}", i).as_str(), Span::call_site());
          let branch_name_str = branch_name.to_string();
          let as_name =
            Ident::new(format!("as_{}", i).as_str(), Span::call_site());
          let full_branch_name = Ident::new(
            format!("{}Variant{}", struct_name, i).as_str(),
            Span::call_site(),
          );
          let branch_type = t.print();
          match branch_type {
            | Some(branch_type) => {
              // TODO: Clean up hard-coding of unit type in unions.
              if branch_type.to_string() == "()" {
                enum_kinds.push(quote! {
                  #branch_name,
                });
                json_arms.push(quote! {
                  | #struct_name::#branch_name => ::serde_json::Value::Null,
                });
                serialize_arms.push(quote! {
                  | #struct_name::#branch_name => ().serialize(serializer),
                });
                as_fns.push(quote! {
                  pub fn #as_name(&self) -> ::core::result::Result<(), ::anyhow::Error> {
                    if let #struct_name::#branch_name = self {
                      ::core::result::Result::Ok(())
                    } else {
                      ::core::result::Result::Err(::anyhow::anyhow!(
                        "Expected variant {}::{}",
                        #struct_name_str,
                        #branch_name_str,
                      ))
                    }
                  }
                });
              } else {
                enum_kinds.push(quote! {
                  #branch_name(#branch_type),
                });
                json_arms.push(quote! {
                  | #struct_name::#branch_name(value) => ::serde_json::json!(value),
                });
                serialize_arms.push(quote! {
                  | #struct_name::#branch_name(ref value) => value.serialize(serializer),
                });
                as_fns.push(quote! {
                  pub fn #as_name(&self) -> ::core::result::Result<#branch_type, ::anyhow::Error> {
                    if let #struct_name::#branch_name(value) = self {
                      ::core::result::Result::Ok(value.clone())
                    } else {
                      ::core::result::Result::Err(::anyhow::anyhow!(
                        "Expected variant {}::{}",
                        #struct_name_str,
                        #branch_name_str,
                      ))
                    }
                  }
                });
              }
            },
            | None => {
              enum_kinds.push(quote! {
                #branch_name(#full_branch_name),
              });
              json_arms.push(quote! {
                | #struct_name::#branch_name(value) => ::serde_json::json!(value),
              });
              serialize_arms.push(quote! {
                | #struct_name::#branch_name(ref value) => value.serialize(serializer),
              });
              // TODO: Probably doing too much cloning.
              as_fns.push(quote! {
                pub fn #as_name(&self) -> ::core::result::Result<#full_branch_name, ::anyhow::Error> {
                  if let #struct_name::#branch_name(value) = self {
                    ::core::result::Result::Ok(value.clone())
                  } else {
                    ::core::result::Result::Err(::anyhow::anyhow!(
                      "Expected variant {}::{}",
                      #struct_name_str,
                      #branch_name_str,
                    ))
                  }
                }
              });
            },
          };
          match t {
            | ConvexType::Id(_) => {
              extract_arms.push(quote! {
                | ::convex::Value::String(value) => {
                  ::core::result::Result::Ok(#struct_name::#branch_name(value.clone()))
                },
              });
            },
            | ConvexType::Null => extract_arms.push(quote! {
              | ::convex::Value::Null => {
                ::core::result::Result::Ok(#struct_name::#branch_name)
              },
            }),
            // TODO: Should this accept Float64 or just Int64?
            | ConvexType::Int64 => extract_arms.push(quote! {
              | ::convex::Value::Int64(value) => {
                ::core::result::Result::Ok(#struct_name::#branch_name(value.clone()))
              },
              | ::convex::Value::Float64(value) => {
                ::core::result::Result::Ok(#struct_name::#branch_name(value.clone() as i64))
              },
            }),
            | ConvexType::IntLiteral(i) => extract_arms.push(quote! {
              | ::convex::Value::Int64(value) if value.clone() == #i => {
                ::core::result::Result::Ok(#struct_name::#branch_name(value.clone()))
              },
              | ::convex::Value::Float64(value) if value.clone() as i64 == #i => {
                ::core::result::Result::Ok(#struct_name::#branch_name(value.clone() as i64))
              },
            }),
            // TODO: Should this accept Int64 or just Float64?
            | ConvexType::Number => extract_arms.push(quote! {
              | ::convex::Value::Int64(value) => {
                ::core::result::Result::Ok(#struct_name::#branch_name(value.clone() as f64))
              },
              | ::convex::Value::Float64(value) => {
                ::core::result::Result::Ok(#struct_name::#branch_name(value.clone()))
              },
            }),
            | ConvexType::Bool => extract_arms.push(quote! {
              | ::convex::Value::Boolean(value) => {
                ::core::result::Result::Ok(#struct_name::#branch_name(value.clone()))
              },
            }),
            | ConvexType::BoolLiteral(b) => extract_arms.push(quote! {
              | ::convex::Value::Boolean(value) if value.clone() == #b => {
                ::core::result::Result::Ok(#struct_name::#branch_name(value.clone()))
              },
            }),
            | ConvexType::String => extract_arms.push(quote! {
              | ::convex::Value::String(value) => {
                ::core::result::Result::Ok(#struct_name::#branch_name(value.clone()))
              },
            }),
            | ConvexType::StringLiteral(s) => extract_arms.push(quote! {
              | ::convex::Value::String(value) if value == #s => {
                ::core::result::Result::Ok(#struct_name::#branch_name(value.clone()))
              },
            }),

            | ConvexType::Object(fields) => {
              structs.append(&mut Self::print_structs(fields, &full_branch_name));
              impls.push(Self::print_from_convex_value(fields, &full_branch_name));
              extract_arms.push(quote! {
                | value if #full_branch_name::from_convex_value(value).is_ok() => {
                  Ok(#struct_name::#branch_name(#full_branch_name::from_convex_value(value)?))
                }
              });
            },

            | ConvexType::Optional(_) => panic!("Unions may not contain optional branches"),
            | ConvexType::Union(_) => panic!("Unions may not directly contain other unions, put other types between them"),
          };
        }

        structs.push(quote! {
          #ignore_attributes
          #enum_struct_attributes
          pub enum #struct_name {
            #( #enum_kinds )*
          }
        });

        impls.push(quote! {
          #ignore_attributes
          impl ::serde::Serialize for #struct_name {
            fn serialize<S>(&self, serializer: S) -> ::core::result::Result<S::Ok, S::Error>
            where S: ::serde::Serializer {
              match *self {
                #( #serialize_arms )*
              }
            }
          }
        });

        impls.push(quote! {
          #ignore_attributes
          impl #struct_name {
            fn from_convex_value(
              value: &::convex::Value
            ) -> ::core::result::Result<Self, ::anyhow::Error> {
              match value {
                #( #extract_arms )*
                | _ => {
                  Err(::anyhow::anyhow!("Invalid union type for '{}'", #field_name_str))
                },
              }
            }

            #(
              #as_fns
            )*
          }
        });

        impls.push(quote! {
          #ignore_attributes
          impl ::core::convert::From<#struct_name> for ::serde_json::Value {
            fn from(value: #struct_name) -> Self {
              match value {
                #( #json_arms )*
              }
            }
          }
        })
      },
      | _ => {
        panic!(
          "Internal Error: Should have a complex field like object or union"
        )
      },
    }

    [structs, impls].concat()
  }

  fn print_from_convex_value(
    fields: &Vec<ConvexField>,
    struct_name: &Ident,
  ) -> TokenStream {
    let ignore_attributes = quote! {
      #[allow(non_snake_case)]
    };
    let extract_ts = Self::print_extract_fields(fields, struct_name);
    quote! {
      #ignore_attributes
      impl #struct_name {
        fn from_convex_value(
          value: &::convex::Value
        ) -> ::core::result::Result<Self, ::anyhow::Error> {
          #extract_ts
        }
      }
    }
  }

  fn print_structs(
    fields: &Vec<ConvexField>,
    struct_name: &Ident,
  ) -> Vec<TokenStream> {
    let ignore_attributes = quote! {
      #[allow(non_snake_case)]
    };
    let struct_attributes = quote! {
      #[derive(::serde::Serialize, ::serde::Deserialize, Clone, Debug, PartialEq)]
    };
    let mut structs = Vec::new();
    let mut rendered_fields = Vec::new();
    for field in fields {
      let field_name = field.name.to_field_name();
      match field.t.print() {
        | Some(field_type) => rendered_fields.push(quote! {
          pub #field_name: #field_type,
        }),
        | None => {
          // TODO: This is hacky hard-codes nesting
          if let ConvexType::Optional(child) = &field.t {
            let struct_name = field.name.to_struct_name();
            let field_type = quote! { Option<#struct_name> };
            let mut child_struct = child.print();
            structs.append(&mut child_struct);
            rendered_fields.push(quote! {
              pub #field_name: #field_type,
            });
          } else {
            let field_type = field.name.to_struct_name();
            let mut child_struct = field.print();
            structs.append(&mut child_struct);
            rendered_fields.push(quote! {
              pub #field_name: #field_type,
            });
          }
        },
      };
    }
    structs.push(quote! {
      #ignore_attributes
      #struct_attributes
      pub struct #struct_name {
        #( #rendered_fields )*
      }
    });
    structs
  }

  fn print_to_json_impl(
    fields: &Vec<ConvexField>,
    struct_name: &Ident,
  ) -> TokenStream {
    let ignore_attributes = quote! {
      #[allow(non_snake_case)]
    };
    let mut json_fields = Vec::new();
    for field in fields {
      let field_name = field.name.to_field_name();
      let field_name_str = field_name.to_string();
      json_fields.push(quote! {
        #field_name_str: value.#field_name,
      });
    }
    quote! {
      #ignore_attributes
      impl ::core::convert::From<#struct_name> for ::serde_json::Value {
        fn from(value: #struct_name) -> Self {
          ::serde_json::json!({
            #( #json_fields )*
          })
        }
      }
    }
  }

  fn print_extract_fields(
    fields: &Vec<ConvexField>,
    struct_name: &Ident,
  ) -> TokenStream {
    let mut extract_fields = Vec::new();
    let mut field_idents = Vec::new();
    for field in fields {
      extract_fields.push(Self::print_extract_field(field, None));
      let field_name = field.name.to_field_name();
      field_idents.push(quote! {
        #field_name,
      });
    }

    quote! {
      match value {
        | ::convex::Value::Object(object) => {
          #( #extract_fields )*

          Ok(#struct_name {
            #( #field_idents )*
          })
        },
        | _ => {
          return Err(::anyhow::anyhow!("Expected an object"));
        },
      }
    }
  }

  fn print_extract_field(
    field: &ConvexField,
    match_ident: Option<Ident>,
  ) -> TokenStream {
    let field_name = field.name.to_field_name();
    let field_name_str = field_name.to_string();
    let match_target = match match_ident {
      | Some(ident) => quote! { #ident },
      | None => quote! { object.get(#field_name_str) },
    };

    match &field.t {
      | ConvexType::Object(_) => {
        let struct_name = field.name.to_struct_name();
        quote! {
          let #field_name = match #match_target {
            | ::core::option::Option::Some(value) => {
              #struct_name::from_convex_value(value)?
            },
            | _ => {
              return Err(::anyhow::anyhow!("Expected '{}' to be an object", #field_name_str));
            },
          };
        }
      },
      | ConvexType::Union(_) => {
        let struct_name = field.name.to_struct_name();
        quote! {
          let #field_name = match #match_target {
            | ::core::option::Option::Some(value) => {
              #struct_name::from_convex_value(value)?
            },
            | _ => {
              return Err(::anyhow::anyhow!("Expected '{}' to match union", #field_name_str));
            },
          };
        }
      },
      | _ => Self::print_extract_type(
        &field.t,
        field_name,
        match_target,
        field_name_str,
      ),
    }
  }

  fn print_extract_type(
    t: &ConvexType,
    ident: Ident,
    match_target: TokenStream,
    error_name: String,
  ) -> TokenStream {
    match &t {
      | ConvexType::Id(_) | ConvexType::String => {
        Self::print_extract_string(ident, match_target, error_name)
      },
      | ConvexType::Null => {
        Self::print_extract_null(ident, match_target, error_name)
      },
      | ConvexType::Int64 => {
        Self::print_extract_int(ident, match_target, error_name)
      },
      | ConvexType::Number => {
        Self::print_extract_number(ident, match_target, error_name)
      },
      | ConvexType::Bool => {
        Self::print_extract_bool(ident, match_target, error_name)
      },
      | ConvexType::IntLiteral(literal) => Self::print_extract_int_literal(
        ident,
        match_target,
        error_name,
        *literal,
      ),
      | ConvexType::BoolLiteral(literal) => Self::print_extract_bool_literal(
        ident,
        match_target,
        error_name,
        *literal,
      ),
      | ConvexType::StringLiteral(literal) => {
        Self::print_extract_string_literal(
          ident,
          match_target,
          error_name,
          literal.into(),
        )
      },

      | ConvexType::Optional(next_t) => {
        let next_target = Ident::new("value", Span::call_site());
        let child_match = Self::print_extract_field(next_t, Some(next_target));
        quote! {
          let #ident = match #match_target {
            | ::core::option::Option::Some(::convex::Value::Null) => ::core::option::Option::None,
            | ::core::option::Option::None => ::core::option::Option::None,
            | value => {
              #child_match
              ::core::option::Option::Some(#ident)
            },
          };
        }
      },
      | _ => {
        panic!("Unimplemented print_extract_type")
      },
    }
  }

  fn print_extract_bool(
    ident: Ident,
    match_target: TokenStream,
    error_name: String,
  ) -> TokenStream {
    quote! {
      let #ident = match #match_target {
        | ::core::option::Option::Some(::convex::Value::Boolean(b)) => b.clone(),
        | _ => {
          return Err(::anyhow::anyhow!("Expected '{}' to be a boolean", #error_name));
        },
      };
    }
  }

  fn print_extract_int(
    ident: Ident,
    match_target: TokenStream,
    error_name: String,
  ) -> TokenStream {
    // TODO: Should this work for both Ints and Floats?
    quote! {
      let #ident = match #match_target {
        | ::core::option::Option::Some(::convex::Value::Int64(i)) => i.clone(),
        | ::core::option::Option::Some(::convex::Value::Float64(f)) => f.clone() as i64,
        | _ => {
          return Err(::anyhow::anyhow!("Expected '{}' to be an int", #error_name));
        },
      };
    }
  }

  fn print_extract_null(
    ident: Ident,
    match_target: TokenStream,
    error_name: String,
  ) -> TokenStream {
    quote! {
      let #ident = match #match_target {
        | ::core::option::Option::Some(::convex::Value::Null) => (),
        | _ => {
          return Err(::anyhow::anyhow!("Expected '{}' to be null", #error_name));
        },
      };
    }
  }

  fn print_extract_string(
    ident: Ident,
    match_target: TokenStream,
    error_name: String,
  ) -> TokenStream {
    quote! {
      let #ident = match #match_target {
        | ::core::option::Option::Some(::convex::Value::String(value)) => value.clone(),
        | _ => {
          return Err(::anyhow::anyhow!("Expected '{}' to be a string", #error_name));
        },
      };
    }
  }

  fn print_extract_int_literal(
    ident: Ident,
    match_target: TokenStream,
    error_name: String,
    literal: i64,
  ) -> TokenStream {
    // TODO: Should this support both float and int?
    quote! {
      let #ident = match #match_target {
        | ::core::option::Option::Some(::convex::Value::Float64(value)) => {
          let v = value.clone() as i64;
          if v != #literal {
            return Err(::anyhow::anyhow!("Expected '{}' to be the int literal '{}'", #error_name, #literal));
          } else {
            v
          }
        },
        | ::core::option::Option::Some(::convex::Value::Int64(value)) => {
          let v = value.clone();
          if v != #literal {
            return Err(::anyhow::anyhow!("Expected '{}' to be the int literal '{}'", #error_name, #literal));
          } else {
            v
          }
        },
        | _ => {
          return Err(::anyhow::anyhow!("Expected '{}' to be an int literal", #error_name));
        },
      };
    }
  }

  fn print_extract_bool_literal(
    ident: Ident,
    match_target: TokenStream,
    error_name: String,
    literal: bool,
  ) -> TokenStream {
    quote! {
      let #ident = match #match_target {
        | ::core::option::Option::Some(::convex::Value::Boolean(value)) => {
          let v = value.clone();
          if v != #literal {
            return Err(::anyhow::anyhow!("Expected '{}' to be the boolean literal '{}'", #error_name, #literal));
          } else {
            v
          }
        },
        | _ => {
          return Err(::anyhow::anyhow!("Expected '{}' to be a boolean literal", #error_name));
        },
      };
    }
  }

  fn print_extract_string_literal(
    ident: Ident,
    match_target: TokenStream,
    error_name: String,
    literal: String,
  ) -> TokenStream {
    quote! {
      let #ident = match #match_target {
        | ::core::option::Option::Some(::convex::Value::String(value)) => {
          let v = value.clone();
          if v != #literal {
            return Err(::anyhow::anyhow!("Expected '{}' to be the string literal '{}'", #error_name, #literal));
          } else {
            v
          }
        },
        | _ => {
          return Err(::anyhow::anyhow!("Expected '{}' to be a string literal", #error_name));
        },
      };
    }
  }

  fn print_extract_number(
    ident: Ident,
    match_target: TokenStream,
    error_name: String,
  ) -> TokenStream {
    // TODO: Should this work for both Ints and Floats?
    quote! {
      let #ident = match #match_target {
        | ::core::option::Option::Some(::convex::Value::Float64(value)) => value.clone(),
        | ::core::option::Option::Some(::convex::Value::Int64(value)) => value.clone() as f64,
        | _ => {
          return Err(::anyhow::anyhow!("Expected '{}' to be a number", #error_name));
        },
      };
    }
  }

  fn parse_child(name: &ConvexName, input: ParseStream) -> Result<Self> {
    // name: v.string(...)
    // ^^^^
    let ident = Ident::parse(input)?;
    let id = ident.to_string();
    let name = ConvexName { path: name.full_path(), id };

    // name: v.string(...)
    //     ^
    let _ = input.parse::<Token![:]>()?;

    // name: v.string(...)
    //       ^^^^^^^^^^^^^
    let t = Self::parse_validator_call(&name, input)?;

    Ok(Self { name, t })
  }

  fn parse_validator_call(
    name: &ConvexName,
    input: ParseStream,
  ) -> Result<ConvexType> {
    // v.string(...)
    // ^
    let v = Ident::parse(input)?;
    if v != "v" {
      return Err(Error::new_spanned(&v, "Expected v.method()"));
    }

    // v.string(...)
    //  ^
    let _ = input.parse::<Token![.]>()?;

    // v.string(...)
    //   ^^^^^^
    let method_ident = Ident::parse(input)?;
    let method = method_ident.to_string();

    // v.string(...)
    //         ^^^^^
    let inner;
    let _ = syn::parenthesized!(inner in input);

    match method.as_str() {
      | "id" => {
        let lit = Lit::parse(&inner)?;
        match lit.clone() {
          | Lit::Str(str_lit) => Ok(ConvexType::Id(str_lit.value())),
          | _ => Err(Error::new_spanned(&lit, "Expected string literal")),
        }
      },
      | "null" => Ok(ConvexType::Null),
      | "int64" => Ok(ConvexType::Int64),
      | "number" => Ok(ConvexType::Number),
      | "boolean" => Ok(ConvexType::Bool),
      | "string" => Ok(ConvexType::String),

      | "literal" => {
        let lit = Lit::parse(&inner)?;
        match lit.clone() {
          | Lit::Str(s) => Ok(ConvexType::StringLiteral(s.value())),
          | Lit::Bool(b) => Ok(ConvexType::BoolLiteral(b.value())),
          | Lit::Int(i) => Ok(ConvexType::IntLiteral(i.base10_parse::<i64>()?)),
          | _ => Err(Error::new_spanned(&lit, "Unsupported literal")),
        }
      },

      | "optional" => {
        let t = Self::parse_validator_call(name, &inner)?;
        Ok(ConvexType::Optional(Box::new(ConvexField {
          name: name.clone(),
          t,
        })))
      },

      | "object" => {
        let object_inner;
        let _ = syn::braced!(object_inner in inner);
        let ts =
          Self::parse_comma_separated(name, &object_inner, |name, b| {
            Self::parse_child(name, b)
          })?;
        Ok(ConvexType::Object(ts))
      },

      | "union" => {
        let ts = Self::parse_comma_separated(name, &inner, |name, b| {
          Self::parse_validator_call(name, b)
        })?;
        if ts.len() < 2 {
          return Err(Error::new_spanned(
            &method_ident,
            "Unions must have 2 or more branches",
          ));
        }
        Ok(ConvexType::Union(ts))
      },

      | _ => {
        Err(Error::new_spanned(&method_ident, "Unsupported validator call"))
      },
    }
  }

  fn parse_comma_separated<T>(
    name: &ConvexName,
    buffer: &ParseBuffer,
    f: fn(&ConvexName, &ParseBuffer) -> Result<T>,
  ) -> Result<Vec<T>> {
    let mut results = Vec::new();
    let mut first = true;
    let mut comma_token = Ok(());
    while !buffer.is_empty() {
      if !first {
        // Must have comma token if this wasn't the first item.
        comma_token?;
      }
      let x = f(name, buffer)?;
      results.push(x);
      comma_token = buffer.parse::<Token![,]>().map(|_| ());
      first = false;
    }
    Ok(results)
  }
}

fn capitalize_first_char(s: &str) -> String {
  s.char_indices().fold(String::new(), |mut acc, (i, c)| {
    if i == 0 {
      acc.extend(c.to_uppercase());
    } else {
      acc.push(c);
    }
    acc
  })
}
