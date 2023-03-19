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
/// same alignment, performance of `AlignedSerializer`, which
/// does respect alignment, is likely to be almost exactly the same.
#[derive(Serializer)]
#[ser_type(pure_copy)]
#[__local]
pub struct UnalignedSerializer<BorrowedStorage: BorrowMut<UnalignedVec>> {
	#[ser_storage(UnalignedVec)]
	storage: BorrowedStorage,
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

	/// Create new `UnalignedSerializer` with pre-allocated storage with capacity
	/// of `capacity` bytes.
	pub fn with_capacity(capacity: usize) -> Self {
		Self {
			storage: UnalignedVec::with_capacity(capacity),
		}
	}
}

impl<BorrowedStorage> UnalignedSerializer<BorrowedStorage>
where BorrowedStorage: BorrowMut<UnalignedVec>
{
	/// Create new `UnalignedSerializer` from an existing
	/// `BorrowMut<UnalignedVec>`.
	pub fn from_storage(storage: BorrowedStorage) -> Self {
		Self { storage }
	}
}