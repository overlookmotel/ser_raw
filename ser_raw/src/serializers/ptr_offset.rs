use std::borrow::BorrowMut;

use crate::{
	pos::PosMapping,
	storage::{AlignedVec, Storage},
	Serializer,
};

/// Serializer that overwrites pointers in output with position offsets,
/// relative to the start of the output buffer.
///
/// Unlike `PureCopySerializer`, this allows a deserializer to walk through the
/// serializer output in any order.
///
/// Values in output will be correctly aligned for their types.
///
/// See [`Storage`] for an explanation of the const parameters.
///
/// # Example
///
/// ```
/// use ser_raw::{
/// 	PtrOffsetSerializer, Serialize, Serializer,
/// 	storage::RandomAccessStorage,
/// 	util::aligned_max_capacity,
/// };
///
/// let boxed: Box<u8> = Box::new(123);
/// const MAX_CAPACITY: usize = aligned_max_capacity(16);
/// let mut ser = PtrOffsetSerializer::<16, 16, 8, MAX_CAPACITY, _>::new();
/// let (pos, storage) = ser.serialize(&boxed);
/// assert_eq!(pos, 0);
///
/// const PTR_SIZE: usize = std::mem::size_of::<usize>();
/// let offset: usize = unsafe { *storage.read(pos) };
/// let value: u8 = unsafe { *storage.read(pos + offset) };
/// assert_eq!(offset, 8);
/// assert_eq!(value, 123);
/// ```
#[derive(Serializer)]
#[ser_type(ptr_offset)]
#[__local]
pub struct PtrOffsetSerializer<
	const STORAGE_ALIGNMENT: usize,
	const MAX_VALUE_ALIGNMENT: usize,
	const VALUE_ALIGNMENT: usize,
	const MAX_CAPACITY: usize,
	BorrowedStorage: BorrowMut<AlignedVec<STORAGE_ALIGNMENT, MAX_VALUE_ALIGNMENT, VALUE_ALIGNMENT, MAX_CAPACITY>>,
> {
	#[ser_storage(AlignedVec<STORAGE_ALIGNMENT, MAX_VALUE_ALIGNMENT, VALUE_ALIGNMENT, MAX_CAPACITY>)]
	storage: BorrowedStorage,
	#[ser_pos_mapping]
	pos_mapping: PosMapping,
}

impl<const SA: usize, const MVA: usize, const VA: usize, const MAX: usize>
	PtrOffsetSerializer<SA, MVA, VA, MAX, AlignedVec<SA, MVA, VA, MAX>>
{
	/// Create new [`PtrOffsetSerializer`] with no memory pre-allocated.
	///
	/// If you know, or can estimate, the amount of buffer space that's going to
	/// be needed in advance, allocating upfront with [`with_capacity`] can
	/// dramatically improve performance vs using `new`.
	///
	/// [`with_capacity`]: PtrOffsetSerializer::with_capacity
	#[inline]
	pub fn new() -> Self {
		Self {
			storage: AlignedVec::new(),
			pos_mapping: PosMapping::dummy(),
		}
	}

	/// Create new [`PtrOffsetSerializer`] with buffer pre-allocated with
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

impl<const SA: usize, const MVA: usize, const VA: usize, const MAX: usize, BorrowedStorage>
	PtrOffsetSerializer<SA, MVA, VA, MAX, BorrowedStorage>
where BorrowedStorage: BorrowMut<AlignedVec<SA, MVA, VA, MAX>>
{
	/// Alignment of output buffer
	pub const STORAGE_ALIGNMENT: usize = SA;

	/// Maximum alignment of values being serialized
	pub const MAX_VALUE_ALIGNMENT: usize = MVA;

	/// Typical alignment of values being serialized
	pub const VALUE_ALIGNMENT: usize = VA;

	/// Maximum capacity of output buffer.
	pub const MAX_CAPACITY: usize = MAX;

	/// Create new [`PtrOffsetSerializer`] from an existing
	/// `BorrowMut<AlignedVec>`.
	pub fn from_storage(storage: BorrowedStorage) -> Self {
		Self {
			storage,
			pos_mapping: PosMapping::dummy(),
		}
	}
}
