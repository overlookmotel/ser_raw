use proc_macro2;
use syn::{parse_macro_input, Data, DeriveInput};

mod structs;
use structs::derive_struct;
mod enums;
use enums::derive_enum;

#[proc_macro_derive(Serialize, attributes(ser_raw_with))]
pub fn serialize(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	serialize_impl(input).into()
}

fn serialize_impl(input: DeriveInput) -> proc_macro2::TokenStream {
	match input.data {
		Data::Struct(data) => derive_struct(data, input.ident, input.generics),
		Data::Enum(data) => derive_enum(data, input.ident, input.generics),
		Data::Union(_) => todo!("Deriving `Serialize` on Unions not supported"),
	}
}
