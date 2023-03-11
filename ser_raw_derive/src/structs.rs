#![allow(unused_imports)]

use proc_macro2::{Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{
	parse_macro_input, parse_quote, spanned::Spanned, Data, DataEnum, DataStruct, DeriveInput, Field,
	Fields, FieldsNamed, FieldsUnnamed, GenericParam, Generics, Ident, Index, Meta, MetaList,
	NestedMeta, Path, TraitBound, TypeParam,
};

pub fn derive_struct(
	data: DataStruct,
	ident: Ident,
	generics: Generics,
	generics_for_impl: Generics,
) -> TokenStream {
	let field_stmts: Vec<TokenStream> = match data.fields {
		Fields::Named(fields) => get_named_field_stmts(fields),
		Fields::Unnamed(fields) => get_unnamed_field_stmts(fields),
		Fields::Unit => vec![],
	};

	let (impl_generics, _, _) = generics_for_impl.split_for_impl();
	let (_, type_generics, where_clause) = generics.split_for_impl();

	quote! {
		#[automatically_derived]
		impl #impl_generics ::ser_raw::Serialize<__Ser, __Store, __Borrowed> for #ident #type_generics #where_clause {
			fn serialize_data(&self, serializer: &mut __Ser) {
				#(#field_stmts)*
			}
		}
	}
}

fn get_named_field_stmts(fields: FieldsNamed) -> Vec<TokenStream> {
	fields
		.named
		.iter()
		.map(|field| {
			let field_name = field.ident.as_ref().expect("Missing field name");
			get_field_stmt(quote! {#field_name}, field)
		})
		.collect()
}

fn get_unnamed_field_stmts(fields: FieldsUnnamed) -> Vec<TokenStream> {
	fields
		.unnamed
		.iter()
		.enumerate()
		.map(|(index, field)| {
			let index = Index::from(index);
			get_field_stmt(quote! {#index}, field)
		})
		.collect()
}

fn get_field_stmt(field_name: TokenStream, field: &Field) -> TokenStream {
	match get_with(field) {
		Some(with) => {
			quote_spanned! {field.span()=>
				<#with as ::ser_raw::SerializeWith::<_, __Ser, __Store, __Borrowed>>::serialize_data_with(
					&self.#field_name, serializer
				);
			}
		}
		None => {
			quote_spanned! {field.span()=>
				::ser_raw::Serialize::<__Ser, __Store, __Borrowed>::serialize_data(
					&self.#field_name, serializer
				);
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
		.filter(|attr| attr.path().is_ident("ser_with"))
		.collect::<Vec<_>>();

	if attrs.len() == 0 {
		return None;
	}

	if attrs.len() != 1 {
		panic!("Cannot have more than 1 `#[ser_with]` attribute on a field");
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
	panic!("`#[ser_with]` needs a path e.g. `#[ser_with(ForeignTypeProxy)]`");
}
