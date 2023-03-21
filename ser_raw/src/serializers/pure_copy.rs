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
/// Any pointers in the input will be copied unchanged. They'll point to the
/// input, and therefore may not remain valid if the input is dropped.
/// Essentially the pointers in output are meaningless.
///
/// A deserializer can still understand and reconstruct the input from this
/// serializer's output, based on its knownledge of the types' layouts, and the
/// determininstic order in which they've been added to the output. However,
/// this requires deserializing the "tree" in order.
///
/// If you need to deserialize in arbitrary order, use [`PtrOffsetSerializer`]
/// or [`CompleteSerializer`] instead.
///
/// See [`AlignedStorage`] for an explanation of the const parameters.
///
/// # Example
///
/// ```
/// use ser_raw::{
/// 	util::aligned_max_capacity,
/// 	PureCopySerializer, Serialize, Serializer,
/// };
///
/// let boxed: Box<u8> = Box::new(123);
/// const MAX_CAPACITY: usize = aligned_max_capacity(16);
/// let mut ser = PureCopySerializer::<16, 16, 8, MAX_CAPACITY, _>::new();
/// let storage = ser.serialize(&boxed);
/// drop(boxed);
/// ```
///
/// The 1st 8 bytes of `storage` will be a pointer pointing to the original
/// `&boxed as *const Box<u8>`. This is not useful data as `boxed` has been
/// dropped.
///
/// [`AlignedStorage`]: crate::storage::AlignedStorage
/// [`PtrOffsetSerializer`]: crate::PtrOffsetSerializer
/// [`CompleteSerializer`]: crate::CompleteSerializer
#[derive(Serializer)]
#[ser_type(pure_copy)]
#[__local]
pub struct PureCopySerializer<
	const STORAGE_ALIGNMENT: usize,
	const MAX_VALUE_ALIGNMENT: usize,
	const VALUE_ALIGNMENT: usize,
	const MAX_CAPACITY: usize,
	BorrowedStorage: BorrowMut<AlignedVec<STORAGE_ALIGNMENT, MAX_VALUE_ALIGNMENT, VALUE_ALIGNMENT, MAX_CAPACITY>>,
> {
	#[ser_storage(AlignedVec<STORAGE_ALIGNMENT, MAX_VALUE_ALIGNMENT, VALUE_ALIGNMENT, MAX_CAPACITY>)]
	storage: BorrowedStorage,
}

impl<const SA: usize, const MVA: usize, const VA: usize, const MAX: usize>
	PureCopySerializer<SA, MVA, VA, MAX, AlignedVec<SA, MVA, VA, MAX>>
{
	/// Create new [`PureCopySerializer`] with no memory pre-allocated.
	///
	/// If you know, or can estimate, the amount of buffer space that's going to
	/// be needed in advance, allocating upfront with [`with_capacity`] can
	/// dramatically improve performance vs using `new`.
	///
	/// [`with_capacity`]: PureCopySerializer::with_capacity
	#[inline]
	pub fn new() -> Self {
		Self {
			storage: AlignedVec::new(),
		}
	}

	/// Create new [`PureCopySerializer`] with buffer pre-allocated with capacity
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
		}
	}
}

impl<const SA: usize, const MVA: usize, const VA: usize, const MAX: usize, BorrowedStorage>
	PureCopySerializer<SA, MVA, VA, MAX, BorrowedStorage>
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

	/// Create new [`PureCopySerializer`] from an existing
	/// `BorrowMut<AlignedVec>`.
	pub fn from_storage(storage: BorrowedStorage) -> Self {
		Self { storage }
	}
}
