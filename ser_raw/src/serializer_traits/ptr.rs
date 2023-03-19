use std::slice;

use crate::{
	pos::{Addr, PosMapping},
	ser_traits::PosTrackingSerializer,
	storage::Storage,
	Serialize,
};

/// Trait for serializers which overwrite pointers in output.
///
/// Used by `CompleteSerializer` and `RelPtrSerializer`, provided by this crate.
pub trait PtrSerializer: PosTrackingSerializer {
	/// Overwrite a pointer in output.
	///
	/// # Safety
	///
	/// * `ptr_pos` must be less than or equal to
	/// 	`capacity - mem::size_of::<usize>()`
	/// 	(i.e. a position which is within the output)
	/// * `target_pos` must be less than or equal to
	/// 	`capacity - mem::size_of_val(value)`
	/// 	where `value` is the value being pointed to.
	///
	/// Some serializers may also impose requirements concerning alignment which
	/// caller must satisfy.
	unsafe fn write_ptr(&mut self, ptr_pos: usize, target_pos: usize) -> ();

	#[inline]
	fn do_serialize_value<T: Serialize<Self>>(&mut self, value: &T) {
		// Align storage, ready to write value
		self.storage_mut().align_for::<T>();

		// Record position mapping for this value
		self.set_pos_mapping(PosMapping::new(value as *const T as usize, self.pos()));

		// Push value to storage.
		// `push_slice_unaligned`'s requirements are satisfied by `align_for::<T>()` and
		// `align_after::<T>()`.
		unsafe {
			self
				.storage_mut()
				.push_slice_unaligned(slice::from_ref(value))
		};
		self.storage_mut().align_after::<T>();

		// Serialize value (which may use the pos mapping we set)
		value.serialize_data(self);
	}

	// Skip recording position mapping here because no further processing of the
	// slice, but still write pointer
	#[inline]
	fn do_push_slice<T>(&mut self, slice: &[T], ptr_addr: Self::Addr) {
		// Align storage, ready to write slice
		self.storage_mut().align_for::<T>();

		// Overwrite pointer with position within output (relative to start of output)
		unsafe { self.write_ptr(self.pos_mapping().pos_for_addr(ptr_addr.addr()), self.pos()) };

		// Push slice to storage.
		// `push_slice_unaligned`'s requirements are satisfied by `align_for::<T>()` and
		// `align_after::<T>()`.
		unsafe { self.storage_mut().push_slice_unaligned(slice) };
		self.storage_mut().align_after::<T>();
	}

	#[inline]
	fn do_push_and_process_slice<T, P: FnOnce(&mut Self)>(
		&mut self,
		slice: &[T],
		ptr_addr: Self::Addr,
		process: P,
	) {
		// Get position mapping before this push
		let pos_mapping_before = *self.pos_mapping();

		// Align storage, ready to write slice
		self.storage_mut().align_for::<T>();

		// Overwrite pointer with position within output (relative to start of output)
		unsafe { self.write_ptr(pos_mapping_before.pos_for_addr(ptr_addr.addr()), self.pos()) };

		// Record position mapping for this slice
		self.set_pos_mapping(PosMapping::new(slice.as_ptr() as usize, self.pos()));

		// Push slice to storage.
		// `push_slice_unaligned`'s requirements are satisfied by `align_for::<T>()` and
		// `align_after::<T>()`.
		unsafe { self.storage_mut().push_slice_unaligned(slice) };
		self.storage_mut().align_after::<T>();

		// Call `process` function (which may use the position mapping we set)
		process(self);

		// Reset position mapping back to as it was before
		self.set_pos_mapping(pos_mapping_before);
	}
}
