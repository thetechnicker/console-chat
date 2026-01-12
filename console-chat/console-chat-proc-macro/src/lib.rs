mod subsetable;

#[proc_macro_derive(Subsetable, attributes(subset, subsetable))]
pub fn subsetable_derive_macro(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    subsetable::subsetable_derive_macro2(item.into())
        .unwrap()
        .into()
}
