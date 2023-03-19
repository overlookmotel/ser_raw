use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Field};

use super::tracking::impl_pos_tracking;

pub fn get_rel_ptr_ser_impl(
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

		// Delegate all methods to `PtrSerializer`'s implementation

		fn serialize_value<T: Serialize<Self>>(&mut self, value: &T) {
			#ns PtrSerializer::do_serialize_value(self, value);
		}

		#[inline]
		fn push_slice<T>(&mut self, slice: &[T], ptr_addr: Self::Addr) {
			#ns PtrSerializer::do_push_slice(self, slice, ptr_addr);
		}

		#[inline]
		fn push_and_process_slice<T, P: FnOnce(&mut Self)>(
			&mut self,
			slice: &[T],
			ptr_addr: Self::Addr,
			process: P,
		) {
			#ns PtrSerializer::do_push_and_process_slice(self, slice, ptr_addr, process);
		}
	}
}

fn get_impls(input: &DeriveInput, fields: &Vec<Field>, ns: &TokenStream) -> TokenStream {
	let pos_tracking_impl = impl_pos_tracking(input, fields, ns);

	let ser = &input.ident;
	let (impl_generics, type_generics, where_clause) = input.generics.split_for_impl();

	quote! {
		#pos_tracking_impl

		const _: () = {
			use #ns {PtrSerializer, RelPtrSerializer};

			#[automatically_derived]
			impl #impl_generics PtrSerializer for #ser #type_generics #where_clause {
				/// Overwrite pointer.
				///
				/// # Safety
				///
				/// * `ptr_pos` and `target_pos` must both sit within bounds of output.
				/// * `target_pos` must be location of a valid value for the type being
				///   pointed to.
				/// * `ptr_pos` must be aligned for a pointer.
				#[inline]
				unsafe fn write_ptr(&mut self, ptr_pos: usize, target_pos: usize) {
					// Delegate to `RelPtrSerializer` implementation
					RelPtrSerializer::do_write_ptr(self, ptr_pos, target_pos);
				}
			}

			#[automatically_derived]
			impl #impl_generics RelPtrSerializer for #ser #type_generics #where_clause {}
		};
	}
}
