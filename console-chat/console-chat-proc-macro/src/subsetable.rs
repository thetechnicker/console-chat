use proc_macro2::Span;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::BTreeMap;
use std::collections::HashMap;
use syn::*;

#[derive(deluxe::ExtractAttributes)]
#[deluxe(attributes(subset))]
struct TargetEnums(#[deluxe(flatten)] Vec<String>);

#[derive(deluxe::ExtractAttributes, Default)]
#[deluxe(attributes(subsetable))]
struct SubsetDefaults {
    extra_fields: HashMap<String, Vec<String>>,
    serialization: HashMap<String, bool>,
}

pub(crate) fn subsetable_derive_macro2(
    item: proc_macro2::TokenStream,
) -> deluxe::Result<proc_macro2::TokenStream> {
    // parse input as DeriveInput using deluxe syn re-exports
    let mut input: DeriveInput = syn::parse2(item)?;

    let defaults: SubsetDefaults =
        deluxe::extract_attributes_optional(&mut input, &deluxe::Errors::new());
    let errors = deluxe::Errors::new();

    let src_ident = input.ident.clone();
    let vis = input.vis.clone();
    let generics = input.generics.clone();

    // ensure enum
    let mut data_enum = match input.data {
        syn::Data::Enum(e) => e,
        _ => return Err(syn::Error::new_spanned(src_ident, "expected enum")),
    };

    let mut targets: BTreeMap<String, (Vec<(syn::Variant, bool)>, bool)> = BTreeMap::new();

    for (key, fields) in defaults.extra_fields.iter() {
        for field in fields {
            match syn::parse_str::<syn::Variant>(field) {
                Ok(variant) => {
                    let target = key.to_string();
                    let entry = targets.entry(target).or_insert((vec![], true));
                    entry.0.push((variant.clone(), true));
                }
                Err(e) => errors.push(Span::call_site(), e),
            }
        }
    }

    if let Some(error) = errors.into_compile_error() {
        //Yes its weird
        return Ok(error);
    }

    for variant in data_enum.variants.iter_mut() {
        let TargetEnums(target_names) = deluxe::extract_attributes(variant)?;
        for target in target_names.into_iter() {
            let entry = targets.entry(target).or_insert((vec![], false));
            entry.0.push((variant.clone(), false));
        }
    }

    // No targets => nothing to emit
    if targets.is_empty() {
        return Ok(TokenStream::new());
    }

    let mut generated = TokenStream::new();

    for (target_name, (variants, has_custom_variants)) in targets.iter() {
        let derive_attrs = if defaults.serialization.get(target_name).is_some_and(|x| !*x) {
            quote! {
                #[derive(Debug, Clone, PartialEq, Display)]
            }
        } else {
            quote! {
               #[derive(Debug, Clone, PartialEq, Display, Serialize, Deserialize)]
            }
        };
        let has_custom_variants = *has_custom_variants;
        let target_ident = format_ident!("{}", target_name);
        // reproduce visibility of source for generated enum
        let enum_vis = vis.clone();

        // variant definitions
        let variant_defs: Vec<TokenStream> = variants
            .iter()
            .map(|v| {
                let v_ident = &v.0.ident;
                let fields = &v.0.fields;
                quote! { #v_ident #fields }
            })
            .collect();

        // From<Target> for Source: destructure and rebuild
        let from_arms: Vec<TokenStream> = variants
            .iter()
            .filter_map(|v| {
                let v_ident = &v.0.ident;
                if v.1 {
                    return None;
                }

                let src_v_ident = quote! {#src_ident::#v_ident};
                Some(match &v.0.fields {
                    syn::Fields::Named(fields_named) => {
                        let ids: Vec<_> = fields_named
                            .named
                            .iter()
                            .map(|f| f.ident.as_ref().unwrap())
                            .collect();

                        if has_custom_variants {
                            quote! { #target_ident::#v_ident { #(#ids),* } => Ok(#src_v_ident { #(#ids),* }) }
                        } else {
                            quote! { #target_ident::#v_ident { #(#ids),* } => #src_v_ident { #(#ids),* } }
                        }
                    }
                    syn::Fields::Unnamed(fields_unnamed) => {
                        let pats: Vec<_> = (0..fields_unnamed.unnamed.len())
                            .map(|i| format_ident!("f{}", i))
                            .collect();
                        if has_custom_variants {
                            quote! { #target_ident::#v_ident ( #(#pats),* ) => Ok(#src_v_ident ( #(#pats),* )) }
                        } else {
                            quote! { #target_ident::#v_ident ( #(#pats),* ) => #src_v_ident ( #(#pats),* ) }
                        }
                    }
                    syn::Fields::Unit =>if has_custom_variants {
                            quote! { #target_ident::#v_ident => Ok(#src_v_ident) }
                    }else{
                            quote! { #target_ident::#v_ident => #src_v_ident }
                    },
                })
            })
            .collect();

        // TryFrom<Source> for Target: pattern match source variants and map or fallthrough
        let try_from_arms: Vec<TokenStream> = variants.iter().filter_map(|v| {
            let v_ident = &v.0.ident;
            if v.1 {
                return None
            }
            Some(match &v.0.fields {
                syn::Fields::Named(fields_named) => {
                    let ids: Vec<_> = fields_named.named.iter().map(|f| f.ident.as_ref().unwrap()).collect();
                    quote! {
                        #src_ident::#v_ident { #(#ids),* } => Ok(#target_ident::#v_ident { #(#ids),* })
                    }
                }
                syn::Fields::Unnamed(fields_unnamed) => {
                    let pats: Vec<_> = (0..fields_unnamed.unnamed.len()).map(|i| format_ident!("f{}", i)).collect();
                    quote! {
                        #src_ident::#v_ident ( #(#pats),* ) => Ok(#target_ident::#v_ident ( #(#pats),* ))
                    }
                }
                syn::Fields::Unit => quote! { #src_ident::#v_ident => Ok(#target_ident::#v_ident) },
            })
        }).collect();

        // attach generics to impls if present (simple clone)
        let (impl_generics, ty_generics, where_clause) = generics.split_for_impl();

        let from_gen = if has_custom_variants {
            quote! {
            impl #impl_generics TryFrom<#target_ident> for #src_ident #ty_generics #where_clause {
                type Error = #target_ident ;
                fn try_from(s: #target_ident) -> Result<#src_ident, #target_ident> {
                    match s {
                        #(#from_arms),*,
                        other => Err(other),
                    }
                }
            }
            }
        } else {
            quote! {
            impl #impl_generics From<#target_ident> for #src_ident #ty_generics #where_clause {
                fn from(s: #target_ident) -> #src_ident {
                    match s {
                        #(#from_arms),*
                    }
                }
            }
            }
        };
        let quote_gen = quote! {
            #derive_attrs
            #enum_vis enum #target_ident {
                #(#variant_defs),*
            }

            #from_gen

            impl #impl_generics std::convert::TryFrom<#src_ident #ty_generics> for #target_ident #where_clause {
                type Error = #src_ident #ty_generics;
                fn try_from(s: #src_ident #ty_generics) -> Result<#target_ident, #src_ident #ty_generics> {
                    match s {
                        #(#try_from_arms),*,
                        other => Err(other),
                    }
                }
            }
        };

        generated.extend(quote_gen);
    }

    //
    let wrapper_variants: Result<Vec<(syn::Variant, syn::Ident)>> = targets
        .into_keys()
        .map(|target| {
            Ok((
                syn::parse_str::<syn::Variant>(&target)?,
                format_ident!("{}", target),
            ))
        })
        .collect();
    let wrapper_variants = wrapper_variants?;

    let wrapper_variant_defs: Vec<_> = wrapper_variants
        .iter()
        .map(|(target_variant, target_ident)| quote! {#target_variant (#target_ident)})
        .collect();

    let src_variant = syn::parse_str::<syn::Variant>(&src_ident.to_string())?;

    let wrapper_ident = format_ident!("{}SubsetWrapper", src_ident);

    let wrapper_variants_mapping: Vec<_> = wrapper_variants
        .iter()
        .map(|(target_variant, target_ident)| {
            quote! {
                impl From<#target_ident> for #wrapper_ident {
                    fn from(o: #target_ident)->#wrapper_ident{
                        #wrapper_ident ::#target_variant (o)
                    }
                }
            }
        })
        .collect();

    let wrapper_gen = quote! {
        #[derive(Debug, Clone, PartialEq, Display)]
        #vis enum #wrapper_ident {
            #(#wrapper_variant_defs),*,
            #src_variant (#src_ident),
        }

        impl From<#src_ident> for #wrapper_ident {
            fn from(o: #src_ident)->#wrapper_ident{
                #wrapper_ident ::#src_variant (o)
            }
        }

        impl<T> From<&T> for #wrapper_ident
        where
            T:Clone+Into<#wrapper_ident>
        {
            fn from(o: &T)->#wrapper_ident{
                o.clone().into()
            }
        }


        #(#wrapper_variants_mapping)*

    };
    generated.extend(wrapper_gen);

    Ok(generated)
}
