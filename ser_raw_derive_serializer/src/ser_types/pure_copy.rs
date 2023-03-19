use proc_macro2::TokenStream;
use quote::quote;

pub fn get_pure_copy_ser_impl(ns: &TokenStream) -> (TokenStream, TokenStream) {
	(get_methods(ns), quote! {})
}

fn get_methods(ns: &TokenStream) -> TokenStream {
	quote! {
		type Addr = #ns pos::NoopAddr;
	}
}
