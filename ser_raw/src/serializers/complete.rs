use std::borrow::BorrowMut;

use crate::{
	pos::{PosMapping, Ptrs},
	storage::{AlignedVec, Storage},
	Serializer,
};

/// Serializer that produces a buffer which is a complete valid representation
/// of the input, which can be cast to a `&T` without any deserialization.
///
/// See [`AlignedStorage`] for an explanation of the const parameters.
///
/// [`AlignedStorage`]: crate::storage::AlignedStorage
// TODO: Set defaults for const params.
// TODO: Reverse order of params - `MAX_VALUE_ALIGNMENT` before `VALUE_ALIGNMENT`.
#[derive(Serializer)]
#[ser_type(complete)]
#[__local]
pub struct CompleteSerializer<
	const STORAGE_ALIGNMENT: usize,
	const VALUE_ALIGNMENT: usize,
	const MAX_VALUE_ALIGNMENT: usize,
	const MAX_CAPACITY: usize,
	BorrowedStorage: BorrowMut<AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT, MAX_CAPACITY>>,
> {
	#[ser_storage(AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT, MAX_CAPACITY>)]
	storage: BorrowedStorage,
	#[ser_pos_mapping]
	pos_mapping: PosMapping,
	#[ser_ptrs]
	ptrs: Ptrs,
}

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize>
	CompleteSerializer<SA, VA, MVA, MAX, AlignedVec<SA, VA, MVA, MAX>>
{
	/// Create new [`CompleteSerializer`] with no memory pre-allocated.
	///
	/// If you know, or can estimate, the amount of buffer space that's going to
	/// be needed in advance, allocating upfront with [`with_capacity`] can
	/// dramatically improve performance vs using `new`.
	///
	/// [`with_capacity`]: CompleteSerializer::with_capacity
	#[inline]
	pub fn new() -> Self {
		Self {
			storage: AlignedVec::new(),
			pos_mapping: PosMapping::dummy(),
			ptrs: Ptrs::new(),
		}
	}

	/// Create new [`CompleteSerializer`] with buffer pre-allocated with capacity
	/// of at least `capacity` bytes.
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
			pos_mapping: PosMapping::dummy(),
			ptrs: Ptrs::new(),
		}
	}
}

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize, BorrowedStorage>
	CompleteSerializer<SA, VA, MVA, MAX, BorrowedStorage>
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

	/// Create new [`CompleteSerializer`] from an existing
	/// `BorrowMut<AlignedVec>`.
	pub fn from_storage(storage: BorrowedStorage) -> Self {
		Self {
			storage,
			pos_mapping: PosMapping::dummy(),
			ptrs: Ptrs::new(),
		}
	}
}
