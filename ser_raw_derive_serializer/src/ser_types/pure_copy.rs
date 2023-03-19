use proc_macro2::TokenStream;
use quote::quote;

pub fn get_pure_copy_ser_impl() -> (TokenStream, TokenStream) {
	(get_methods(), quote! {})
}

fn get_methods() -> TokenStream {
	quote! {
		type Addr = _ser_raw::pos::NoopAddr;
	}
}
