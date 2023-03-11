use std::borrow::BorrowMut;

use crate::{
	storage::{Storage, UnalignedVec},
	Serializer,
};

/// Serializer which does not respect alignment in the output.
///
/// Values are likely not be aligned as their types require.
///
/// If most of the allocated types you're serializing share the
/// same alignment, performance of `BaseSerializer`, which
/// does respect alignment, is likely to be almost exactly the same.
pub struct UnalignedSerializer<Store: BorrowMut<UnalignedVec>> {
	storage: Store,
}

impl UnalignedSerializer<UnalignedVec> {
	/// Create new `UnalignedSerializer` without allocating any memory for output
	/// buffer. Memory will be allocated when first value is serialized.
	///
	/// If you know, or can estimate, the amount of buffer space that's going to
	/// be needed in advance, allocating upfront with `with_capacity` can
	/// dramatically improve performance vs `new`.
	pub fn new() -> Self {
		Self {
			storage: UnalignedVec::new(),
		}
	}

	/// Create new `UnalignedSerializer` with buffer pre-allocated with capacity
	/// of `capacity` bytes.
	pub fn with_capacity(capacity: usize) -> Self {
		Self {
			storage: UnalignedVec::with_capacity(capacity),
		}
	}
}

impl<Store: BorrowMut<UnalignedVec>> UnalignedSerializer<Store> {
	/// Create new `UnalignedSerializer` from an existing
	/// `BorrowMut<UnalignedVec>`.
	pub fn from_storage(storage: Store) -> Self {
		Self { storage }
	}

	/// Consume Serializer and return the output buffer as a
	/// `BorrowMut<UnalignedVec>`.
	pub fn into_storage(self) -> Store {
		self.storage
	}
}

impl<Store: BorrowMut<UnalignedVec>> Serializer for UnalignedSerializer<Store> {
	/// Push a slice of values into output buffer.
	#[inline]
	fn push_and_process_slice<T, P: FnOnce(&mut Self)>(&mut self, slice: &[T], process: P) {
		self.push_raw_slice(slice);
		process(self);
	}

	/// Push raw bytes to output.
	/// Slightly optimized implementation which uses
	/// `Vec<u8>::extend_from_slice()`.
	#[inline]
	fn push_bytes(&mut self, bytes: &[u8]) {
		self.storage.borrow_mut().push_bytes(bytes);
	}

	/// Push a slice of values to output.
	#[inline]
	fn push_raw_slice<T>(&mut self, slice: &[T]) {
		self.storage.borrow_mut().push_slice(slice);
	}

	/// Get current capacity of output.
	#[inline]
	fn capacity(&self) -> usize {
		self.storage.borrow().capacity()
	}

	/// Get current position in output.
	#[inline]
	fn pos(&self) -> usize {
		self.storage.borrow().len()
	}
}
