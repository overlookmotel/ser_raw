use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Field};

use super::pos_tracking::impl_pos_tracking;

pub fn get_ptr_offset_ser_impl(
	input: &DeriveInput,
	fields: &Vec<Field>,
) -> (TokenStream, TokenStream) {
	(get_methods(), get_impls(input, fields))
}

fn get_methods() -> TokenStream {
	quote! {
		// Pointer-writing serializers need a functional `Addr`
		type Addr = _ser_raw::pos::TrackingAddr;

		fn serialize_value<T: _ser_raw::Serialize<Self>>(&mut self, value: &T) -> usize {
			// Delegate to `PosTracking` trait's implementation
			ser_traits::PosTracking::do_serialize_value(self, value)
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
	}
}

fn get_impls(input: &DeriveInput, fields: &Vec<Field>) -> TokenStream {
	let pos_tracking_impl = impl_pos_tracking(input, fields);

	let ser = &input.ident;
	let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

	quote! {
		#pos_tracking_impl

		const _: () = {
			use ser_traits::{PtrOffset, PtrWriting};

			#[automatically_derived]
			impl #impl_generics PtrWriting for #ser #type_generics #where_clause {
				/// Overwrite pointer.
				///
				/// # Safety
				///
				/// * `ptr_pos` and `target_pos` must both sit within bounds of output.
				/// * `target_pos` must be location of a valid value for the type being
				///   pointed to.
				/// * `ptr_pos` must be aligned for a pointer.
				#[inline]
				unsafe fn overwrite_ptr(&mut self, ptr_pos: usize, target_pos: usize) {
					// Delegate to `PtrOffset` trait's implementation
					PtrOffset::do_overwrite_ptr(self, ptr_pos, target_pos);
				}
			}

			#[automatically_derived]
			impl #impl_generics PtrOffset for #ser #type_generics #where_clause {}
		};
	}
}
