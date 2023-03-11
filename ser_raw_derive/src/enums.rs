use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{DataEnum, Fields, FieldsNamed, FieldsUnnamed, Generics, Ident};

// TODO: Handle `ser_with` attribute

pub fn derive_enum(
	data: DataEnum,
	ident: Ident,
	generics: Generics,
	generics_for_impl: Generics,
) -> TokenStream {
	let num_variants = data.variants.len();

	let mut matches = data
		.variants
		.into_iter()
		.filter_map(|variant| {
			match variant.fields {
				Fields::Unit => None,
				Fields::Unnamed(fields) => get_match_for_unnamed_fields(variant.ident, fields),
				Fields::Named(fields) => get_match_for_named_fields(variant.ident, fields),
			}
		})
		.collect::<Vec<_>>();

	let match_stmt = if matches.len() == 0 {
		quote! {}
	} else {
		if matches.len() < num_variants {
			matches.push(quote! {
				_ => {}
			});
		}

		quote! {
			match self {
				#(#matches)*
			}
		}
	};

	let (impl_generics, _, _) = generics_for_impl.split_for_impl();
	let (_, type_generics, where_clause) = generics.split_for_impl();

	quote! {
		#[automatically_derived]
		impl #impl_generics ::ser_raw::Serialize<__Ser, __Store, __Borrowed> for #ident #type_generics #where_clause {
			fn serialize_data(&self, serializer: &mut __Ser) {
				#match_stmt
			}
		}
	}
}

fn get_match_for_unnamed_fields(ident: Ident, fields: FieldsUnnamed) -> Option<TokenStream> {
	let fields = fields.unnamed;
	if fields.len() == 0 {
		return None;
	}

	let field_idents = (0..fields.len())
		.into_iter()
		.map(|index| Ident::new(&("val_".to_string() + &index.to_string()), ident.span()))
		.collect::<Vec<_>>();
	let stmts = get_field_stmts(&field_idents);

	Some(quote_spanned! {ident.span()=>
		Self::#ident(#(#field_idents),*) => {
			#(#stmts)*
		}
	})
}

fn get_match_for_named_fields(ident: Ident, fields: FieldsNamed) -> Option<TokenStream> {
	let fields = fields.named;
	if fields.len() == 0 {
		return None;
	}

	let field_idents = fields
		.into_iter()
		.map(|field| field.ident.unwrap())
		.collect::<Vec<_>>();

	// Aliases are required in case of a field called `serializer`.
	// `Self::Foo {x: val_x} =>` instead of just `Self::Foo {x} =>`.
	let field_aliases = field_idents
		.iter()
		.map(|ident| Ident::new(&("val_".to_string() + &ident.to_string()), ident.span()))
		.collect::<Vec<_>>();

	let var_mappings = std::iter::zip(&field_idents, &field_aliases)
		.map(|(ident, alias)| quote! { #ident: #alias })
		.collect::<Vec<_>>();

	let stmts = get_field_stmts(&field_aliases);

	Some(quote_spanned! {ident.span()=>
		Self::#ident{#(#var_mappings),*} => {
			#(#stmts)*
		}
	})
}

fn get_field_stmts(idents: &Vec<Ident>) -> Vec<TokenStream> {
	idents
		.iter()
		.map(|ident| {
			quote! {
				::ser_raw::Serialize::<__Ser, __Store, __Borrowed>::serialize_data(#ident, serializer);
			}
		})
		.collect::<Vec<_>>()
}
