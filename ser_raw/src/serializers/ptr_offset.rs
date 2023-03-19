use std::borrow::BorrowMut;

use crate::{
	pos::PosMapping,
	storage::{AlignedVec, Storage},
	Serialize, Serializer,
};

/// Serializer that overwrites pointers in output with position offsets,
/// relative to the start of the output buffer.
///
/// Unlike `PureCopySerializer`, this allows a deserializer to walk through the
/// serializer output in any order.
///
/// Values in output will be correctly aligned for their types.
///
/// See `AlignedStorage` for an explanation of the const parameters.
///
/// # Example
///
/// ```
/// use ser_raw::{
/// 	PtrOffsetSerializer, Serialize, Serializer,
/// 	storage::ContiguousStorage
/// };
///
/// let boxed: Box<u8> = Box::new(123);
/// let mut ser = PtrOffsetSerializer::<16, 8, 16, 1024, _>::new();
/// let storage = ser.serialize(&boxed);
/// let slice = storage.as_slice();
///
/// const PTR_SIZE: usize = std::mem::size_of::<usize>();
/// let offset = usize::from_ne_bytes(slice[..PTR_SIZE].try_into().unwrap());
/// assert_eq!(offset, 8);
/// assert_eq!(slice[offset], 123);
/// ```
// TODO: Set defaults for const params.
// TODO: Reverse order of params - `MAX_VALUE_ALIGNMENT` before `VALUE_ALIGNMENT`.
#[derive(Serializer)]
#[ser_type(ptr_offset)]
#[__local]
pub struct PtrOffsetSerializer<
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
}

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize>
	PtrOffsetSerializer<SA, VA, MVA, MAX, AlignedVec<SA, VA, MVA, MAX>>
{
	/// Create new `PtrOffsetSerializer` with no memory pre-allocated.
	///
	/// If you know, or can estimate, the amount of buffer space that's going to
	/// be needed in advance, allocating upfront with `with_capacity` can
	/// dramatically improve performance vs using `new`.
	#[inline]
	pub fn new() -> Self {
		Self {
			storage: AlignedVec::new(),
			pos_mapping: PosMapping::dummy(),
		}
	}

	/// Create new `PtrOffsetSerializer` with buffer pre-allocated with
	/// capacity of at least `capacity` bytes.
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
		}
	}
}

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize, BorrowedStorage>
	PtrOffsetSerializer<SA, VA, MVA, MAX, BorrowedStorage>
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

	/// Create new `PtrOffsetSerializer` from an existing
	/// `BorrowMut<AlignedVec>`.
	pub fn from_storage(storage: BorrowedStorage) -> Self {
		Self {
			storage,
			pos_mapping: PosMapping::dummy(),
		}
	}
}