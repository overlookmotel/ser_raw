use proc_macro2;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(Serialize)]
pub fn serialize(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	serialize_impl(input).into()
}

fn serialize_impl(_input: DeriveInput) -> proc_macro2::TokenStream {
	quote! {}
}
