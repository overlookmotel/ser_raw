use std::borrow::BorrowMut;

use crate::{
	storage::{Storage, UnalignedVec},
	InstantiableSerializer, Serializer,
};

/// Serializer which does not respect alignment in the output.
///
/// Values are likely not be aligned as their types require.
///
/// If most of the allocated types you're serializing share the
/// same alignment, performance of `AlignedSerializer`, which
/// does respect alignment, is likely to be almost exactly the same.
pub struct UnalignedSerializer<BorrowedStore: BorrowMut<UnalignedVec>> {
	storage: BorrowedStore,
}

impl InstantiableSerializer<UnalignedVec> for UnalignedSerializer<UnalignedVec> {
	/// Create new `UnalignedSerializer` without allocating any memory for output
	/// buffer. Memory will be allocated when first value is serialized.
	///
	/// If you know, or can estimate, the amount of buffer space that's going to
	/// be needed in advance, allocating upfront with `with_capacity` can
	/// dramatically improve performance vs `new`.
	fn new() -> Self {
		Self {
			storage: UnalignedVec::new(),
		}
	}

	/// Create new `UnalignedSerializer` with pre-allocated storage with capacity
	/// of `capacity` bytes.
	fn with_capacity(capacity: usize) -> Self {
		Self {
			storage: UnalignedVec::with_capacity(capacity),
		}
	}
}

impl<BorrowedStore> Serializer<UnalignedVec, BorrowedStore> for UnalignedSerializer<BorrowedStore>
where BorrowedStore: BorrowMut<UnalignedVec>
{
	/// Get immutable ref to `UnalignedVec` backing this serializer.
	#[inline]
	fn storage(&self) -> &UnalignedVec {
		self.storage.borrow()
	}

	/// Get mutable ref to `UnalignedVec` backing this serializer.
	#[inline]
	fn storage_mut(&mut self) -> &mut UnalignedVec {
		self.storage.borrow_mut()
	}

	/// Create new `UnalignedSerializer` from an existing
	/// `BorrowMut<UnalignedVec>`.
	fn from_storage(storage: BorrowedStore) -> Self {
		Self { storage }
	}

	/// Consume Serializer and return the output buffer as a
	/// `BorrowMut<UnalignedVec>`.
	fn into_storage(self) -> BorrowedStore {
		self.storage
	}
}
