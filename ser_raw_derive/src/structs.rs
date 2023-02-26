#![allow(unused_imports)]

use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{
	parse_macro_input, spanned::Spanned, Data, DataEnum, DataStruct, DeriveInput, Field, Fields,
	FieldsNamed, Generics, Ident, Meta, MetaList, NestedMeta, Path,
};

pub fn derive_struct(data: DataStruct, ident: Ident, generics: Generics) -> TokenStream {
	let field_stmts: Vec<TokenStream> = match data.fields {
		Fields::Named(fields) => get_named_field_stmts(fields),
		Fields::Unnamed(_fields) => todo!("Unnamed struct fields not supported yet"),
		Fields::Unit => todo!("Unit struct fields not supported yet"),
	};

	let (impl_generics, type_generics, where_clause) = generics.split_for_impl();

	quote! {
		#[automatically_derived]
		impl #impl_generics ::ser_raw::Serialize for #ident #type_generics #where_clause {
			fn serialize_data<S: ::ser_raw::Serializer>(&self, serializer: &mut S) {
				#(#field_stmts)*
			}
		}
	}
}

fn get_named_field_stmts(fields: FieldsNamed) -> Vec<TokenStream> {
	fields
		.named
		.iter()
		.map(|field| get_named_field_stmt(field))
		.collect()
}

fn get_named_field_stmt(field: &Field) -> TokenStream {
	let field_name = field.ident.as_ref().expect("Missing field name");
	match get_with(field) {
		Some(with) => {
			quote_spanned! {field.span()=>
				<#with as ::ser_raw::SerializeWith::<_>>::serialize_data_with(&self.#field_name, serializer);
			}
		}
		None => {
			quote_spanned! {field.span()=>
					::ser_raw::Serialize::serialize_data(&self.#field_name, serializer);
			}
		}
	}
}

fn get_with(field: &Field) -> Option<Path> {
	let attrs = field
		.attrs
		.iter()
		.map(|attr| attr.parse_meta())
		.filter_map(Result::ok)
		.filter(|attr| attr.path().is_ident("ser_raw_with"))
		.collect::<Vec<_>>();

	if attrs.len() == 0 {
		return None;
	}

	if attrs.len() != 1 {
		panic!("Cannot have more than 1 `#[ser_raw_with]` attribute on a field");
	}

	let attr = attrs.into_iter().nth(0).unwrap();
	if let Meta::List(MetaList { nested, .. }) = attr {
		let parts: Vec<NestedMeta> = nested.into_iter().collect();
		if parts.len() == 1 {
			let first = parts.into_iter().nth(0).unwrap();
			if let NestedMeta::Meta(Meta::Path(with)) = first {
				return Some(with);
			}
		}
	}
	panic!("`#[ser_raw_with]` needs a path e.g. `#[ser_raw_with(ForeignTypeProxy)]`");
}
