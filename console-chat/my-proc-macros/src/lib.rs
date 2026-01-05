mod from_hashmap;
use from_hashmap::from_hashmap_derive_macro2;

#[proc_macro_derive(FromHashmap, attributes(hashmap))]
pub fn from_hashmap_derive_macro(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    from_hashmap_derive_macro2(item.into()).unwrap().into()
}
