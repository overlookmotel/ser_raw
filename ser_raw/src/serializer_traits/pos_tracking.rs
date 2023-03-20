use std::slice;

use crate::{pos::PosMapping, storage::Storage, Serialize, Serializer};

/// Trait for serializers which track position in output.
///
/// Used by `CompleteSerializer` and `PtrOffsetSerializer`, provided by this
/// crate.
pub trait PosTracking: Serializer {
	// NB: Position tracking serializers can use `NoopAddr` as `Addr` associated
	// type, unless they are also recording pointers.

	/// Get current position mapping
	fn pos_mapping(&self) -> &PosMapping;

	/// Set current position mapping
	fn set_pos_mapping(&mut self, pos_mapping: PosMapping) -> ();

	/// Get position for a value
	#[inline]
	fn pos_for<T>(&self, value: &T) -> usize {
		self.pos_mapping().pos_for(value)
	}

	fn do_serialize_value<T: Serialize<Self>>(&mut self, value: &T) {
		// Align storage, ready to write value
		self.storage_mut().align_for::<T>();

		// Record position mapping for this value
		self.set_pos_mapping(PosMapping::new(value as *const T as usize, self.pos()));

		// Push value to storage.
		// `push_slice_unaligned`'s requirements are satisfied by `align_for::<T>()` and
		// `align_after::<T>()`.
		let slice = slice::from_ref(value);
		unsafe { self.storage_mut().push_slice_unaligned(slice) };
		self.storage_mut().align_after::<T>();

		// Serialize value (which may use the pos mapping we set)
		value.serialize_data(self);
	}

	// Skip recording position when no further processing for a slice
	#[inline]
	fn do_push_slice<T>(&mut self, slice: &[T], _ptr_addr: Self::Addr) {
		self.push_raw_slice(slice);
	}

	#[inline]
	fn do_push_and_process_slice<T, P: FnOnce(&mut Self)>(
		&mut self,
		slice: &[T],
		_ptr_addr: Self::Addr,
		process: P,
	) {
		// Get position mapping before processing this
		let pos_mapping_before = *self.pos_mapping();

		// Align storage, ready to write slice
		self.storage_mut().align_for::<T>();

		// Record position mapping for this slice
		self.set_pos_mapping(PosMapping::new(slice.as_ptr() as usize, self.pos()));

		// Push slice to storage.
		// `push_slice_unaligned`'s requirements are satisfied by `align_for::<T>()` and
		// `align_after::<T>()`.
		unsafe { self.storage_mut().push_slice_unaligned(slice) };
		self.storage_mut().align_after::<T>();

		// Call `process` function (which may use the pos mapping we set)
		process(self);

		// Reset position mapping back to as it was
		self.set_pos_mapping(pos_mapping_before);
	}
}
