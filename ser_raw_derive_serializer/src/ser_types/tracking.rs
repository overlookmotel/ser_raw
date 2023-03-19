use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Field};

use crate::common::get_tagged_field;

pub fn get_tracking_ser_impl(
	input: &DeriveInput,
	fields: &Vec<Field>,
	ns: &TokenStream,
) -> (TokenStream, TokenStream) {
	(get_methods(ns), impl_pos_tracking(input, fields, ns))
}

fn get_methods(ns: &TokenStream) -> TokenStream {
	quote! {
		// Position tracking serializers don't need a functional `Addr`
		type Addr = #ns pos::NoopAddr;

		// Delegate all methods to `PosTrackingSerializer`'s implementation

		#[inline]
		fn serialize_value<T: Serialize<Self>>(&mut self, value: &T) {
			#ns PosTrackingSerializer::do_serialize_value(self, value);
		}

		#[inline]
		fn push_slice<T>(&mut self, slice: &[T], ptr_addr: Self::Addr) {
			#ns PosTrackingSerializer::do_push_slice(self, slice, ptr_addr);
		}

		#[inline]
		fn push_and_process_slice<T, P: FnOnce(&mut Self)>(
			&mut self,
			slice: &[T],
			ptr_addr: Self::Addr,
			process: P,
		) {
			#ns PosTrackingSerializer::do_push_and_process_slice(self, slice, ptr_addr, process);
		}
	}
}

/// Implement `PosTrackingSerializer` trait
pub fn impl_pos_tracking(
	input: &DeriveInput,
	fields: &Vec<Field>,
	ns: &TokenStream,
) -> TokenStream {
	let (pos_mapping, ..) = get_tagged_field(fields, "ser_pos_mapping");

	let ser = &input.ident;
	let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

	quote! {
		const _: () = {
			use #ns {PosTrackingSerializer, pos::PosMapping};

			#[automatically_derived]
			impl #impl_generics PosTrackingSerializer for #ser #type_generics #where_clause {
				/// Get current position mapping
				#[inline]
				fn pos_mapping(&self) -> &PosMapping {
					&self.#pos_mapping
				}

				/// Set current position mapping
				#[inline]
				fn set_pos_mapping(&mut self, pos_mapping: PosMapping) {
					self.#pos_mapping = pos_mapping;
				}
			}
		};
	}
}
