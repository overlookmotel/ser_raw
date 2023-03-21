use std::borrow::BorrowMut;

use crate::{
	storage::{Storage, UnalignedVec},
	Serializer,
};

/// Simple serializer that just copies values, with no position tracking or
/// pointer correction, without respecting alignment.
///
/// Unlike [`PureCopySerializer`], [`UnalignedSerializer`] does not respect
/// alignment in the output. Values are likely not be aligned as their types
/// require.
///
/// If most of the allocated types you're serializing share the
/// same alignment, performance of [`PureCopySerializer`], which
/// does respect alignment, is likely to be almost exactly the same.
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
/// # Example
///
/// ```
/// use ser_raw::{UnalignedSerializer, Serialize, Serializer};
///
/// let boxed: Box<u8> = Box::new(123);
/// let mut ser = UnalignedSerializer::new();
/// let storage = ser.serialize(&boxed);
/// drop(boxed);
/// ```
///
/// The 1st 8 bytes of `storage` will be a pointer pointing to the original
/// `&boxed as *const Box<u8>`. This is not useful data as `boxed` has been
/// dropped.
///
/// [`PureCopySerializer`]: crate::PureCopySerializer
/// [`PtrOffsetSerializer`]: crate::PtrOffsetSerializer
/// [`CompleteSerializer`]: crate::CompleteSerializer
#[derive(Serializer)]
#[ser_type(pure_copy)]
#[__local]
pub struct UnalignedSerializer<BorrowedStorage: BorrowMut<UnalignedVec>> {
	#[ser_storage(UnalignedVec)]
	storage: BorrowedStorage,
}

impl UnalignedSerializer<UnalignedVec> {
	/// Create new [`UnalignedSerializer`] without allocating any memory for
	/// output buffer. Memory will be allocated when first value is serialized.
	///
	/// If you know, or can estimate, the amount of buffer space that's going to
	/// be needed in advance, allocating upfront with [`with_capacity`] can
	/// dramatically improve performance vs `new`.
	///
	/// [`with_capacity`]: UnalignedSerializer::with_capacity
	pub fn new() -> Self {
		Self {
			storage: UnalignedVec::new(),
		}
	}

	/// Create new [`UnalignedSerializer`] with pre-allocated storage with
	/// capacity of `capacity` bytes.
	pub fn with_capacity(capacity: usize) -> Self {
		Self {
			storage: UnalignedVec::with_capacity(capacity),
		}
	}
}

impl<BorrowedStorage> UnalignedSerializer<BorrowedStorage>
where BorrowedStorage: BorrowMut<UnalignedVec>
{
	/// Create new [`UnalignedSerializer`] from an existing
	/// `BorrowMut<UnalignedVec>`.
	pub fn from_storage(storage: BorrowedStorage) -> Self {
		Self { storage }
	}
}
