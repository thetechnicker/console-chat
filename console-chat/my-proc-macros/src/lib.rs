mod from_hashmap;
use from_hashmap::from_hashmap_derive_macro2;

/// Example:
///
///```
///#[derive(serde::Serialize, serde::Deserialize, FromHashmap, Default)]
///#[hashmap(type = "Theme")]
///struct HomeTheme {
///    #[hashmap(default = "DARK_GRAY")]
///    pub root: Theme,
///    #[hashmap(default = "GREEN")]
///    pub join: Theme,
///    #[hashmap(default = "CYAN")]
///    pub random: Theme,
///    #[hashmap(default = "BLUE")]
///    pub login: Theme,
///    #[hashmap(default = "GRAY")]
///    pub settings: Theme,
///    #[hashmap(default = "GRAY")]
///    pub raw_settings: Theme,
///    #[hashmap(default = "GRAY")]
///    pub reset_config: Theme,
///    #[hashmap(default = "RED")]
///    pub exit: Theme,
///    pub inserted: bool,
///}
///```
#[proc_macro_derive(FromHashmap, attributes(hashmap))]
pub fn from_hashmap_derive_macro(item: proc_macro::TokenStream) -> proc_macro::TokenStream {
    from_hashmap_derive_macro2(item.into()).unwrap().into()
}
