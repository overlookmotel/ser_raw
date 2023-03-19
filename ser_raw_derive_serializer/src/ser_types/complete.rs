use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Field};

use super::pos_tracking::impl_pos_tracking;
use crate::common::get_tagged_field;

pub fn get_complete_ser_impl(
	input: &DeriveInput,
	fields: &Vec<Field>,
) -> (TokenStream, TokenStream) {
	(get_methods(), get_impls(input, fields))
}

fn get_methods() -> TokenStream {
	quote! {
		// Pointer-writing serializers need a functional `Addr`
		type Addr = _ser_raw::pos::TrackingAddr;

		fn serialize_value<T: Serialize<Self>>(&mut self, value: &T) {
			// Delegate to `PtrWriting` trait's implementation
			ser_traits::PtrWriting::do_serialize_value(self, value);
		}

		#[inline]
		fn push_slice<T>(&mut self, slice: &[T], ptr_addr: Self::Addr) {
			// Delegate to `PtrWriting` trait's implementation
			ser_traits::PtrWriting::do_push_slice(self, slice, ptr_addr);
		}

		#[inline]
		fn push_and_process_slice<T, P: FnOnce(&mut Self)>(
			&mut self,
			slice: &[T],
			ptr_addr: Self::Addr,
			process: P,
		) {
			// Delegate to `PtrWriting` trait's implementation
			ser_traits::PtrWriting::do_push_and_process_slice(self, slice, ptr_addr, process);
		}

		#[inline]
		unsafe fn write<T>(&mut self, value: &T, addr: usize) {
			// Delegate to `Writable` trait's implementation
			ser_traits::Writable::do_write(self, value, addr);
		}

		#[inline]
		fn write_correction<W: FnOnce(&mut Self)>(&mut self, write: W) {
			// Delegate to `Complete` trait's implementation
			ser_traits::Complete::do_write_correction(self, write);
		}

		#[inline]
		fn finalize(self) -> Self::BorrowedStorage {
			// Delegate to `Complete` trait's implementation
			ser_traits::Complete::do_finalize(self)
		}
	}
}

fn get_impls(input: &DeriveInput, fields: &Vec<Field>) -> TokenStream {
	let pos_tracking_impl = impl_pos_tracking(input, fields);

	let (ptrs, ..) = get_tagged_field(fields, "ser_ptrs");

	let ser = &input.ident;
	let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

	quote! {
		#pos_tracking_impl

		const _: () = {
			use ser_traits::{Complete, Ptrs, PtrWriting, Writable};

			#[automatically_derived]
			impl #impl_generics PtrWriting for #ser #type_generics #where_clause {
				#[inline]
				unsafe fn write_ptr(&mut self, ptr_pos: usize, target_pos: usize) {
					// Delegate to `Complete` trait's implementation
					Complete::do_write_ptr(self, ptr_pos, target_pos);
				}
			}

			#[automatically_derived]
			impl #impl_generics Writable for #ser #type_generics #where_clause {}

			#[automatically_derived]
			impl #impl_generics Complete for #ser #type_generics #where_clause {
				#[inline]
				fn ptrs(&self) -> &Ptrs {
					&self.#ptrs
				}

				#[inline]
				fn ptrs_mut(&mut self) -> &mut Ptrs {
					&mut self.#ptrs
				}
			}
		};
	}
}
