use std::borrow::BorrowMut;

use crate::{
	storage::{AlignedVec, Storage},
	Serializer,
};

/// Simple serializer that just copies values, with no position tracking or
/// pointer correction.
///
/// Values in output will be correctly aligned for their types.
///
/// See `AlignedStorage` for an explanation of the const parameters.
// TODO: Set defaults for const params.
// TODO: Reverse order of params - `MAX_VALUE_ALIGNMENT` before `VALUE_ALIGNMENT`.
#[derive(Serializer)]
#[ser_type(pure_copy)]
#[__local]
pub struct PureCopySerializer<
	const STORAGE_ALIGNMENT: usize,
	const VALUE_ALIGNMENT: usize,
	const MAX_VALUE_ALIGNMENT: usize,
	const MAX_CAPACITY: usize,
	BorrowedStorage: BorrowMut<AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT, MAX_CAPACITY>>,
> {
	#[ser_storage(AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT, MAX_CAPACITY>)]
	storage: BorrowedStorage,
}

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize>
	PureCopySerializer<SA, VA, MVA, MAX, AlignedVec<SA, VA, MVA, MAX>>
{
	/// Create new `PureCopySerializer` with no memory pre-allocated.
	///
	/// If you know, or can estimate, the amount of buffer space that's going to
	/// be needed in advance, allocating upfront with `with_capacity` can
	/// dramatically improve performance vs using `new`.
	#[inline]
	pub fn new() -> Self {
		Self {
			storage: AlignedVec::new(),
		}
	}

	/// Create new `PureCopySerializer` with buffer pre-allocated with capacity of
	/// at least `capacity` bytes.
	///
	/// `capacity` will be rounded up to a multiple of `MAX_VALUE_ALIGNMENT`.
	///
	/// # Panics
	///
	/// Panics if `capacity` exceeds `MAX_CAPACITY`.
	pub fn with_capacity(capacity: usize) -> Self {
		// `AlignedVec::with_capacity()` ensures capacity is `< MAX_CAPACITY`
		// and rounds up capacity to a multiple of `MAX_VALUE_ALIGNMENT`
		Self {
			storage: AlignedVec::with_capacity(capacity),
		}
	}
}

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize, BorrowedStorage>
	PureCopySerializer<SA, VA, MVA, MAX, BorrowedStorage>
where BorrowedStorage: BorrowMut<AlignedVec<SA, VA, MVA, MAX>>
{
	/// Alignment of output buffer
	pub const STORAGE_ALIGNMENT: usize = SA;

	/// Typical alignment of values being serialized
	pub const VALUE_ALIGNMENT: usize = VA;

	/// Maximum alignment of values being serialized
	pub const MAX_VALUE_ALIGNMENT: usize = MVA;

	/// Maximum capacity of output buffer.
	pub const MAX_CAPACITY: usize = MAX;

	/// Create new `PureCopySerializer` from an existing `BorrowMut<AlignedVec>`.
	pub fn from_storage(storage: BorrowedStorage) -> Self {
		Self { storage }
	}
}
