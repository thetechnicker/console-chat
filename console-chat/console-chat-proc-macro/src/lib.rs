mod subsetable;

#[proc_macro_derive(Subsetable, attributes(subset, subsetable))]
pub fn subsetable_derive_macro(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    subsetable::subsetable_derive_macro2(item.into())
        .unwrap()
        .into()
}

#[proc_macro]
pub fn validate_url(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    use url::Url;
    // Parse the input token stream as a string literal
    let input_str = syn::parse_macro_input!(input as syn::LitStr);
    let value = input_str.value();

    // Validation logic: Check if the string starts with a capital letter
    let _ = Url::parse(&value).expect("default URL should be valid; this is a source code bug");

    // If valid, return the string as a TokenStream
    //let output = quote::quote! { Url::parse(#input_str).unwrap(); };
    let output = quote::quote! { #input_str };
    proc_macro::TokenStream::from(output)
}
