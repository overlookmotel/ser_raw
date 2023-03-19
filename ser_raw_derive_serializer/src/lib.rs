use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse2, parse_macro_input, DeriveInput, Field, Ident, Type};

pub(crate) mod common;
use common::{get_fields, get_namespace, get_ser_type, get_tagged_field, SerializerType};
mod ser_types;
use ser_types::{
	get_complete_ser_impl, get_pure_copy_ser_impl, get_rel_ptr_ser_impl, get_tracking_ser_impl,
};

#[proc_macro_derive(
	Serializer,
	attributes(ser_type, ser_storage, ser_pos_mapping, ser_ptrs, __local)
)]
pub fn serializer(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
	let input = parse_macro_input!(input as DeriveInput);
	serializer_impl(input).into()
}

fn serializer_impl(input: DeriveInput) -> TokenStream {
	// Get serializer type
	let ser_type = get_ser_type(&input);

	// Find storage field
	let fields = get_fields(&input);
	let (storage_field_name, storage_type, borrowed_storage_type) = get_storage_field(&fields);

	// Get extra methods, associated types and impls depending on serializer type
	let ns = get_namespace(&input);
	let (methods_and_types, impls) = match ser_type {
		SerializerType::PureCopy => get_pure_copy_ser_impl(&ns),
		SerializerType::Tracking => get_tracking_ser_impl(&input, &fields, &ns),
		SerializerType::RelPtr => get_rel_ptr_ser_impl(&input, &fields, &ns),
		SerializerType::Complete => get_complete_ser_impl(&input, &fields, &ns),
	};

	// Implement `Serializer`
	let ser = &input.ident;
	let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

	quote! {
		const _: () = {
			use ::std::borrow::{Borrow, BorrowMut};
			#[allow(unused_imports)]
			use #ns {Serializer, ser_traits};

			#[automatically_derived]
			impl #impl_generics Serializer for #ser #type_generics #where_clause {
				type Storage = #storage_type;
				type BorrowedStorage = #borrowed_storage_type;

				#methods_and_types

				#[inline]
				fn storage(&self) -> &#storage_type {
					self.#storage_field_name.borrow()
				}

				#[inline]
				fn storage_mut(&mut self) -> &mut #storage_type {
					self.#storage_field_name.borrow_mut()
				}

				#[inline]
				fn into_storage(self) -> #borrowed_storage_type {
					self.#storage_field_name
				}
			}
		};

		#impls
	}
}

fn get_storage_field(fields: &Vec<Field>) -> (Ident, Type, Type) {
	let (field_name, borrowed_ty, attr) = get_tagged_field(fields, "ser_storage");

	let ty = parse2::<Type>(attr.tokens).expect(
		"`ser_storage` attribute must include single storage type `#[ser_storage(StorageType)]`",
	);
	let ty = match ty {
		Type::Paren(ty) => *ty.elem,
		_ => unreachable!(),
	};

	(field_name, ty, borrowed_ty)
}
