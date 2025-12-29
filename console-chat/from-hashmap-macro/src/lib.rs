use std::collections::HashMap;
use syn::DeriveInput;

#[derive(deluxe::ExtractAttributes)]
#[deluxe(attributes(hashmap))]
struct StructAttributes {
    r#type: String,
}

#[derive(deluxe::ExtractAttributes)]
#[deluxe(attributes(hashmap))]
struct FieldAttributes {
    name: Option<String>,
    default: String,
    #[deluxe(default = false)]
    inserted: bool,
}

fn parse_field_attributes(
    ast: &mut DeriveInput,
) -> deluxe::Result<HashMap<String, (syn::Ident, FieldAttributes)>> {
    let mut field_attrs = HashMap::new();
    if let syn::Data::Struct(s) = &mut ast.data {
        for field in s.fields.iter_mut() {
            let field_ident = field.ident.as_ref().unwrap().clone();
            let field_name = field_ident.to_string();
            let attrs: FieldAttributes = deluxe::extract_attributes(field)?;
            if attrs.inserted {
                field_attrs.insert("INSERTED".to_string(), (field_ident, attrs));
                continue;
            }
            field_attrs.insert(field_name, (field_ident, attrs));
        }
    }
    Ok(field_attrs)
}

fn from_hashmap_derive_macro2(
    item: proc_macro2::TokenStream,
) -> deluxe::Result<proc_macro2::TokenStream> {
    let mut ast: DeriveInput = syn::parse2(item)?;

    let StructAttributes { r#type } = deluxe::extract_attributes(&mut ast)?;

    let mut field_attrs = parse_field_attributes(&mut ast)?;
    let (inserted_field_ident, _) = field_attrs.remove("INSERTED").unwrap();
    let (ensure_field, set_value): (Vec<proc_macro2::TokenStream>, Vec<proc_macro2::TokenStream>) =
        field_attrs
            .iter()
            .filter_map(|(name, (ident, attr))| {
                let default_value = attr.default.clone();
                let default_exp: syn::Expr = syn::parse_str(&default_value).ok()?;
                let name = attr.name.as_ref().unwrap_or(name).to_string();
                let a = quote::quote! {
                    ensure!(map, #name, #default_exp, inserted);
                };
                let b = quote::quote! {
                    #ident: map.get(#name).cloned().unwrap_or(#default_exp)
                };
                Some((a, b))
            })
            .unzip();

    let ident = ast.ident;
    let (impl_generics, type_generics, where_clause) = ast.generics.split_for_impl();
    let expected_type: syn::Type = syn::parse_str(&r#type)?;

    Ok(quote::quote! {
        impl #impl_generics From<&mut std::collections::HashMap<String, #expected_type>> for #ident #type_generics #where_clause {
            fn from(map: &mut std::collections::HashMap<String, #expected_type>)->#ident{
                macro_rules! ensure {
                    ($map:expr, $key:expr, $default:expr, $flag:ident) => {{
                        if !$map.contains_key($key) {
                            $map.insert($key.to_string(), $default);
                            $flag = true;
                        }
                    }};
                }

                let mut inserted = false;
                #(#ensure_field)*

                #ident {
                    #(#set_value),*
                    #inserted_field_ident: inserted,
                }
            }
        }
    })
}

#[proc_macro_derive(FromHashmap, attributes(hashmap))]
pub fn from_hashmap_derive_macro(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    from_hashmap_derive_macro2(item.into()).unwrap().into()
}
