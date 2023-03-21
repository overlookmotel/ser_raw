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
/// # Safety
///
/// It's always safe to serialize, but casting the serializer's output back to a
/// `&T` can be very unsafe unless the following warnings are heeded.
///
/// The format of serialized output is dependent on the system serialization is
/// performed on. This format is **not** portable to other systems with
/// different architectures (big endian vs little endian, 32 bit vs 64 bit, or
/// possibly other differences in system architecture).
///
/// Rust also offers no guarantee that even the same code compiled twice on the
/// same system will result in the same memory layouts (in practice it does, but
/// you can always tag your types `#[repr(C)]` to make sure).
///
/// Therefore, great care should be taken to ensure deserialization occurs on
/// same type of machine as serialization occured on, and ideally using the same
/// binary. A mismatch will be very likely to cause memory unsafety and the
/// dreaded *undefined behavior*.
///
/// Additionally, the storage's backing buffer must not move in memory after
/// serialization. Adding more data to the storage later (e.g. with
/// `storage.push_bytes()`) may exceed the storage's capacity, causing it to
/// grow and reallocate to a different memory location. However, the pointers in
/// the storage buffer will still point to the old memory locations, which are
/// no longer valid. Accessing the deserialized value will then be UB.
///
/// # Example
///
/// ```
/// use ser_raw::{
/// 	CompleteSerializer, Serialize, Serializer,
/// 	storage::ContiguousStorage,
/// 	util::aligned_max_capacity,
/// };
///
/// let boxed: Box<u8> = Box::new(123);
///
/// // Serialize
/// const MAX_CAPACITY: usize = aligned_max_capacity(16);
/// let mut ser = CompleteSerializer::<16, 16, 8, MAX_CAPACITY, _>::new();
/// let storage = ser.serialize(&boxed);
///
/// // Deserialize
/// // This is safe because:
/// // 1. Serialization and deserialization are performed
/// //    on same system with same binary
/// // 2. `storage` has not been mutated after serialization completed
/// let boxed_out: &Box<u8> = unsafe { &*storage.as_ptr().cast() };
/// assert_eq!(boxed_out, &boxed);
/// ```
///
/// [`AlignedStorage`]: crate::storage::AlignedStorage
// TODO: Set defaults for const params.
#[derive(Serializer)]
#[ser_type(complete)]
#[__local]
pub struct CompleteSerializer<
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
	#[ser_ptrs]
	ptrs: Ptrs,
}

impl<const SA: usize, const MVA: usize, const VA: usize, const MAX: usize>
	CompleteSerializer<SA, MVA, VA, MAX, AlignedVec<SA, MVA, VA, MAX>>
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

impl<const SA: usize, const MVA: usize, const VA: usize, const MAX: usize, BorrowedStorage>
	CompleteSerializer<SA, MVA, VA, MAX, BorrowedStorage>
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
