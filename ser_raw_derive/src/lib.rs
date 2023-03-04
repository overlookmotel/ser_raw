use proc_macro2;
use syn::{
	parse_macro_input, parse_quote, Attribute, Data, DeriveInput, GenericParam, Generics, Ident,
	TraitBound,
};

mod structs;
use structs::derive_struct;
mod enums;
use enums::derive_enum;

#[proc_macro_derive(Serialize, attributes(ser_with, ser_type, ser_bound))]
pub fn serialize(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	serialize_impl(input).into()
}

fn serialize_impl(input: DeriveInput) -> proc_macro2::TokenStream {
	let generics = input.generics;
	let (ser_type, generics_for_impl) = get_generics(input.attrs, &generics);

	match input.data {
		Data::Struct(data) => derive_struct(data, input.ident, ser_type, generics, generics_for_impl),
		Data::Enum(data) => derive_enum(data, input.ident, ser_type, generics, generics_for_impl),
		Data::Union(_) => todo!("Deriving `Serialize` on Unions not supported"),
	}
}

/// Amend generics to add Serializer trait bound
fn get_generics(attrs: Vec<Attribute>, generics: &Generics) -> (Ident, Generics) {
	// Parse attributes for user-specified serializer type / trait bound
	// with `#[ser_type]` or `#[ser_bound]`
	let (ser_type, ser_bound) = get_options(attrs);

	// Assemble serializer type + trait bound.
	// Use `ser_type` or `ser_bound` if provided,
	// otherwise default bound: `__S: ::ser_raw::Serializer`.
	let mut generics_for_impl = generics.clone();
	let ser_type = match ser_type {
		Some(ser_ident) => ser_ident,
		None => {
			let ser_ident: Ident = parse_quote!(__S);
			let ser_bound = ser_bound.unwrap_or(parse_quote!(::ser_raw::Serializer));
			generics_for_impl
				.params
				.push(GenericParam::Type(parse_quote!(#ser_ident: #ser_bound)));
			ser_ident
		}
	};

	(ser_type, generics_for_impl)
}

fn get_options(attrs: Vec<Attribute>) -> (Option<Ident>, Option<TraitBound>) {
	let mut ser_type: Option<Ident> = None;
	let mut ser_bound: Option<TraitBound> = None;

	for attr in attrs {
		if attr.path.is_ident("ser_type") {
			let id = attr
				.parse_args::<Ident>()
				.expect("Malformed `ser_type` attr");
			if ser_type.is_some() {
				panic!("Can only have one `#[ser_type]` attribute");
			}
			ser_type = Some(id);
		} else if attr.path.is_ident("ser_bound") {
			let bound = attr
				.parse_args::<TraitBound>()
				.expect("Malformed `ser_bound` attr");
			if ser_bound.is_some() {
				panic!("Can only have one `#[ser_bound]` attribute");
			}
			ser_bound = Some(bound);
		}
	}

	if ser_type.is_some() && ser_bound.is_some() {
		panic!("Cannot use `#[ser_type]` and `#[ser_bound]` attributes together");
	}

	(ser_type, ser_bound)
}
