#![allow(unused_imports)]
#![allow(unused_variables)]

use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DataEnum, DataStruct, DeriveInput, Generics, Ident};

pub fn derive_enum(data: DataEnum, ident: Ident) -> TokenStream {
	todo!("Enums not supported yet");
}
