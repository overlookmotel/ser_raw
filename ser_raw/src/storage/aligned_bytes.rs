use std::ptr::NonNull;

/// Aligned storage with a set capacity which cannot grow.
pub struct AlignedBytes {
	#[allow(dead_code)] // TODO: Remove this once implemented
	ptr: NonNull<u8>,
	capacity: usize,
	len: usize,
}

impl AlignedBytes {
	/// Create new [`AlignedBytes`] with no allocated memory.
	///
	/// Only useful as a placeholder, as it can't grow and therefore isn't able to
	/// store anything!
	pub fn new() -> Self {
		Self {
			ptr: NonNull::dangling(),
			capacity: 0,
			len: 0,
		}
	}

	/// Create new [`AlignedBytes`] with pre-allocated capacity.
	///
	/// Capacity is set in stone. It cannot grow beyond this size.
	pub fn with_capacity(capacity: usize) -> Self {
		// TODO: Actually allocate some memory!
		Self {
			ptr: NonNull::dangling(),
			capacity,
			len: 0,
		}
	}

	/// Returns capacity of this [`AlignedBytes`] in bytes.
	pub fn capacity(&self) -> usize {
		self.capacity
	}

	/// Returns amount of storage currently used in this [`AlignedBytes`] in
	/// bytes.
	pub fn len(&self) -> usize {
		self.len
	}

	#[allow(unused_variables)] // TODO: Delete once implemented
	pub unsafe fn push_slice_unchecked<T>(&mut self, slice: &[T], size: usize) {
		// TODO
	}

	// TODO: Implement all `Storage` methods
}

impl Default for AlignedBytes {
	fn default() -> Self {
		// Dummy empty storage
		Self::new()
	}
}
