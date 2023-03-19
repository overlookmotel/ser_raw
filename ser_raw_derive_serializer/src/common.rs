use proc_macro2::TokenStream;
use quote::quote;
use syn::{
	Attribute, Data, DataStruct, DeriveInput, Field, Fields, FieldsNamed, Ident, Meta, MetaList,
	NestedMeta, Type,
};

pub enum SerializerType {
	PureCopy,
	Tracking,
	RelPtr,
	Complete,
}

/// Get type of serializer to be implemented from `#[ser_type]` attribute
pub fn get_ser_type(input: &DeriveInput) -> SerializerType {
	let attrs = input
		.attrs
		.iter()
		.filter(|attr| attr.path.is_ident("ser_type"))
		.collect::<Vec<_>>();
	if attrs.len() == 0 {
		panic!("`#[ser_type]` attribute is required");
	} else if attrs.len() > 1 {
		panic!("Found more than 1 `#[ser_type]` attribute");
	}

	let ser_type = match attrs[0].parse_meta() {
		Ok(meta) => {
			match meta {
				Meta::List(MetaList { nested, .. }) => {
					match nested.len() {
						1 => {
							match &nested[0] {
								NestedMeta::Meta(Meta::Path(path)) => {
									path.get_ident().map(|ident| ident.to_string())
								}
								_ => None,
							}
						}
						_ => None,
					}
				}
				_ => None,
			}
		}
		_ => None,
	}
	.expect("`ser_type` attribute must include serializer type e.g. `#[ser_type(pure_copy)]`");

	match ser_type.as_ref() {
		"pure_copy" => SerializerType::PureCopy,
		"tracking" => SerializerType::Tracking,
		"rel_ptr" => SerializerType::RelPtr,
		"complete" => SerializerType::Complete,
		_ => {
			panic!(
				"Unrecognised `#[ser_type]` type. Valid options are 'pure_copy', 'tracking', 'rel_ptr', \
				 'complete'"
			);
		}
	}
}

/// Get namespace for hygienic references to `ser_raw`'s exports.
/// Usually this will be `::ser_raw`.
/// However, `::ser_raw::Serializer` doesn't work within `ser_raw` crate itself.
/// When macros are used within `ser_raw`'s own codebase, the structs are tagged
/// `#[__local]`, and then a `crate` namespace is used instead.
pub fn get_namespace(input: &DeriveInput) -> TokenStream {
	let is_local = input
		.attrs
		.iter()
		.filter(|attr| attr.path.is_ident("__local"))
		.next()
		.is_some();
	match is_local {
		true => quote! {crate},
		false => quote! {::ser_raw},
	}
}

/// Get struct's fields.
/// Ensure input is a struct with named fields.
pub fn get_fields(input: &DeriveInput) -> Vec<Field> {
	let fields = match &input.data {
		Data::Struct(DataStruct { fields, .. }) => fields,
		_ => panic!("`#[derive(Serializer)]` is only valid on structs"),
	};
	let fields = match fields {
		Fields::Named(FieldsNamed { named, .. }) => named,
		_ => {
			panic!("`#[derive(Serializer)]` is only valid on structs with named fields")
		}
	};
	fields.iter().cloned().collect::<Vec<_>>()
}

/// Get struct's field tagged with `#[<tag>]`.
/// Panics if no tag found, or more than one tag found.
/// Returns the field name, field type, and attribute.
pub fn get_tagged_field(fields: &Vec<Field>, tag: &str) -> (Ident, Type, Attribute) {
	let filtered_fields = fields
		.into_iter()
		.filter_map(|field| {
			let attrs = field
				.attrs
				.iter()
				.filter(|attr| attr.path.is_ident(tag))
				.collect::<Vec<_>>();
			if attrs.len() == 0 {
				None
			} else if attrs.len() > 1 {
				panic!("Only 1 `#[{}]` attribute can be used on a field", tag);
			} else {
				Some((field, attrs.into_iter().nth(0).unwrap()))
			}
		})
		.collect::<Vec<_>>();

	if filtered_fields.len() == 0 {
		panic!("One of struct's fields must have a `#[{}]` attribute", tag);
	} else if filtered_fields.len() > 1 {
		panic!(
			"Only one of struct's fields can have a `#[{}]` attribute",
			tag
		);
	}

	let (field, attr) = filtered_fields.into_iter().nth(0).unwrap();
	let field_name = field.ident.clone().unwrap();

	(field_name, field.ty.clone(), attr.clone())
}
