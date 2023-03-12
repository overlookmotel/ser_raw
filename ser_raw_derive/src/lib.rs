use proc_macro2;
use syn::{
	parse_macro_input, parse_quote, Attribute, Data, DeriveInput, GenericParam, Generics, TraitBound,
};

mod structs;
use structs::derive_struct;
mod enums;
use enums::derive_enum;

#[proc_macro_derive(Serialize, attributes(ser_with, ser_bound))]
pub fn serialize(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	serialize_impl(input).into()
}

fn serialize_impl(input: DeriveInput) -> proc_macro2::TokenStream {
	let generics = input.generics;
	let generics_for_impl = get_generics(input.attrs, &generics);

	match input.data {
		Data::Struct(data) => derive_struct(data, input.ident, generics, generics_for_impl),
		Data::Enum(data) => derive_enum(data, input.ident, generics, generics_for_impl),
		Data::Union(_) => todo!("Deriving `Serialize` on Unions not supported"),
	}
}

/// Amend generics to add Serializer trait bound
fn get_generics(attrs: Vec<Attribute>, generics: &Generics) -> Generics {
	// Parse attributes for user-specified serializer bound `#[ser_bound]`
	let ser_bound = get_ser_bound(attrs);

	// Add bounds for serializer + storage.
	// Add bound from `#[ser_bound(...)]` to Serializer if present.
	let mut generic_param: GenericParam = parse_quote!(__S: ::ser_raw::Serializer);
	if let Some(ser_bound) = ser_bound {
		generic_param = parse_quote!(#generic_param + #ser_bound);
	}

	let mut generics_for_impl = generics.clone();
	generics_for_impl.params.push(generic_param);
	generics_for_impl
}

fn get_ser_bound(attrs: Vec<Attribute>) -> Option<TraitBound> {
	let mut ser_bound: Option<TraitBound> = None;
	for attr in attrs {
		if attr.path.is_ident("ser_bound") {
			let bound = attr
				.parse_args::<TraitBound>()
				.expect("Malformed `ser_bound` attr");
			if ser_bound.is_some() {
				panic!("Can only have one `#[ser_bound]` attribute");
			}
			ser_bound = Some(bound);
		}
	}
	ser_bound
}
