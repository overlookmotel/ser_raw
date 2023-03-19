use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Field};

use super::tracking::impl_pos_tracking;
use crate::common::get_tagged_field;

pub fn get_complete_ser_impl(
	input: &DeriveInput,
	fields: &Vec<Field>,
	ns: &TokenStream,
) -> (TokenStream, TokenStream) {
	(get_methods(ns), get_impls(input, fields, ns))
}

fn get_methods(ns: &TokenStream) -> TokenStream {
	quote! {
		// Pointer-writing serializers need a functional `Addr`
		type Addr = #ns pos::TrackingAddr;

		fn serialize_value<T: Serialize<Self>>(&mut self, value: &T) {
			// Delegate to `PtrSerializer`'s implementation
			#ns PtrSerializer::do_serialize_value(self, value);
		}

		#[inline]
		fn push_slice<T>(&mut self, slice: &[T], ptr_addr: Self::Addr) {
			// Delegate to `PtrSerializer`'s implementation
			#ns PtrSerializer::do_push_slice(self, slice, ptr_addr);
		}

		#[inline]
		fn push_and_process_slice<T, P: FnOnce(&mut Self)>(
			&mut self,
			slice: &[T],
			ptr_addr: Self::Addr,
			process: P,
		) {
			// Delegate to `PtrSerializer`'s implementation
			#ns PtrSerializer::do_push_and_process_slice(self, slice, ptr_addr, process);
		}

		#[inline]
		unsafe fn write<T>(&mut self, value: &T, addr: usize) {
			// Delegate to `WritableSerializer`'s implementation
			#ns WritableSerializer::do_write(self, value, addr);
		}

		#[inline]
		fn write_correction<W: FnOnce(&mut Self)>(&mut self, write: W) {
			// Delegate to `CompleteSerializerTrait`'s implementation
			#ns CompleteSerializerTrait::do_write_correction(self, write);
		}

		#[inline]
		fn finalize(self) -> Self::BorrowedStorage {
			// Delegate to `CompleteSerializerTrait`'s implementation
			#ns CompleteSerializerTrait::do_finalize(self)
		}
	}
}

fn get_impls(input: &DeriveInput, fields: &Vec<Field>, ns: &TokenStream) -> TokenStream {
	let pos_tracking_impl = impl_pos_tracking(input, fields, ns);

	let (ptrs_record, ..) = get_tagged_field(fields, "ser_ptrs");

	let ser = &input.ident;
	let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

	quote! {
		#pos_tracking_impl

		const _: () = {
			use #ns {CompleteSerializerTrait, PtrsRecord, PtrSerializer, WritableSerializer};

			#[automatically_derived]
			impl #impl_generics PtrSerializer for #ser #type_generics #where_clause {
				#[inline]
				unsafe fn write_ptr(&mut self, ptr_pos: usize, target_pos: usize) {
					// Delegate to `CompleteSerializerTrait`'s implementation
					CompleteSerializerTrait::do_write_ptr(self, ptr_pos, target_pos);
				}
			}

			#[automatically_derived]
			impl #impl_generics WritableSerializer for #ser #type_generics #where_clause {}

			#[automatically_derived]
			impl #impl_generics CompleteSerializerTrait for #ser #type_generics #where_clause {
				#[inline]
				fn ptrs_record(&self) -> &PtrsRecord {
					&self.#ptrs_record
				}

				#[inline]
				fn ptrs_record_mut(&mut self) -> &mut PtrsRecord {
					&mut self.#ptrs_record
				}
			}
		};
	}
}
