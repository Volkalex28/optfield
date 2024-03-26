use quote::quote;
use syn::spanned::Spanned;
use syn::{parse2, Error, Field, Fields, ItemStruct, Path, Result, Type, TypePath};

use crate::args::{self, Args};
use crate::error::unexpected;

mod attrs;

const OPTION: &str = "Option";

/// Wraps item fields in Option.
pub fn generate(item: &ItemStruct, args: &Args) -> Result<Fields> {
    let item_name = item.ident.clone();

    let mut fields = item.fields.clone();

    if let Some(args_fields) = args.fields.as_ref() {
        use syn::{Fields::*, FieldsNamed as FN, FieldsUnnamed as FU};
        match args_fields {
            args::Fields::Add(new) => match (&mut fields, new.clone()) {
                (_, Unit) => {}
                (fields @ Unit, new) => *fields = new,
                (Unnamed(FU { unnamed: fds, .. }), new @ Unnamed(_))
                | (Named(FN { named: fds, .. }), new @ Named(_)) => fds.extend(new.into_iter()),
                (Named(_), new) => return Err(Error::new(new.span(), "Expected named fields")),
                (Unnamed(_), new) => return Err(Error::new(new.span(), "Expected unnamed fields")),
            },
        }
    }

    for field in fields.iter_mut() {
        field.attrs = attrs::generate(field, args);
        attrs::generate(field, args);

        if let Some(vis) = args.vis.as_ref() {
            field.vis = vis.clone()
        }

        if is_option(field) && !args.rewrap {
            continue;
        }

        let ty = &field.ty;

        let opt_type = quote! {
            Option<#ty>
        };

        field.ty = parse2(opt_type).map_err(|e| {
            Error::new(
                e.span(),
                unexpected(format!("generating {} fields", item_name), e),
            )
        })?;
    }

    Ok(fields)
}

pub fn is_option(field: &Field) -> bool {
    match &field.ty {
        Type::Path(TypePath {
            path: Path { segments, .. },
            ..
        }) => {
            if let Some(segment) = segments.first() {
                segment.ident == OPTION
            } else {
                false
            }
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::test_util::*;

    #[test]
    fn test_is_not_option() {
        let field = parse_field(quote! {
            field: String
        });

        assert!(!is_option(&field));
    }

    #[test]
    fn test_is_option() {
        let field = parse_field(quote! {
            field: Option<String>
        });

        assert!(is_option(&field));
    }

    #[test]
    fn without_rewrap() {
        let (item, args) = parse_item_and_args(
            quote! {
                struct S<T> {
                    string: Option<String>,
                    int: i32,
                    generic: T,
                    optional_generic: Option<T>
                }
            },
            quote! {
                Opt
            },
        );

        let expected_types = parse_types(vec![
            quote! {Option<String>},
            quote! {Option<i32>},
            quote! {Option<T>},
            quote! {Option<T>},
        ]);

        let generated = generate(&item, &args);

        assert_eq!(field_types(generated), expected_types);
    }

    #[test]
    fn with_rewrap() {
        let (item, args) = parse_item_and_args(
            quote! {
                struct S<T> {
                    text: String,
                    number: Option<i128>,
                    generic: T,
                    optional_generic: Option<T>
                }
            },
            quote! {
                Opt,
                rewrap
            },
        );

        let expected_types = parse_types(vec![
            quote! {Option<String>},
            quote! {Option<Option<i128>>},
            quote! {Option<T>},
            quote! {Option<Option<T>>},
        ]);

        let generated = generate(&item, &args);

        assert_eq!(field_types(generated), expected_types);
    }
}
