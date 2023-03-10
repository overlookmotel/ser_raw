use std::{borrow::BorrowMut, mem, slice};

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
	/// Create new Serializer without allocating any memory for output buffer.
	/// Memory will be allocated when first object is serialized.
	pub fn new() -> Self {
		Self {
			storage: UnalignedVec::new(),
		}
	}

	/// Create new Serializer with buffer pre-allocated with capacity of
	/// `capacity` bytes.
	///
	/// If you know, or can estimate, the amount of buffer space that's going to
	/// be needed in advance, allocating upfront with `with_capacity` can
	/// dramatically improve performance vs `new`.
	pub fn with_capacity(capacity: usize) -> Self {
		Self {
			storage: UnalignedVec::with_capacity(capacity),
		}
	}
}

impl<Store: BorrowMut<UnalignedVec>> UnalignedSerializer<Store> {
	/// Create new Serializer from an existing `UnalignedVec`
	/// or `&mut UnalignedVec`.
	pub fn from_store(storage: Store) -> Self {
		Self { storage }
	}

	/// Consume Serializer and return the output buffer as an `UnalignedVec`
	/// or `&mut UnalignedVec`.
	pub fn into_store(self) -> Store {
		self.storage
	}
}

impl<Store: BorrowMut<UnalignedVec>> Serializer for UnalignedSerializer<Store> {
	#[inline]
	fn push_slice<T>(&mut self, slice: &[T]) {
		self.push_raw_slice(slice);
	}

	#[inline]
	fn push_bytes(&mut self, bytes: &[u8]) {
		self.storage.borrow_mut().extend_from_slice(bytes);
	}

	#[inline]
	fn push_raw_slice<T>(&mut self, slice: &[T]) {
		let ptr = slice.as_ptr() as *const u8;
		let bytes = unsafe { slice::from_raw_parts(ptr, slice.len() * mem::size_of::<T>()) };
		self.push_bytes(bytes);
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

	/// Move current position in output buffer.
	///
	/// # Safety
	///
	/// * `pos` must be less than or equal to `self.capacity()`.
	#[inline]
	unsafe fn set_pos(&mut self, pos: usize) {
		self.storage.borrow_mut().set_len(pos);
	}
}
