use std::borrow::BorrowMut;

use crate::{
	impl_pure_copy_serializer,
	storage::{AlignedVec, Storage},
	BorrowingSerializer, InstantiableSerializer, PureCopySerializer, SerializerStorage,
};

/// Serializer that ensures values are correctly aligned in output buffer.
///
/// See `AlignedStorage` for an explanation of the const parameters.
pub struct AlignedSerializer<
	const STORAGE_ALIGNMENT: usize,
	const VALUE_ALIGNMENT: usize,
	const MAX_VALUE_ALIGNMENT: usize,
	const MAX_CAPACITY: usize,
	BorrowedStore: BorrowMut<AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT, MAX_CAPACITY>>,
> {
	storage: BorrowedStore,
}

// Expose const params as associated consts - `Self::STORAGE_ALIGNMENT` etc.
impl<
		const STORAGE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize,
		BorrowedStore: BorrowMut<AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT, MAX_CAPACITY>>,
	>
	AlignedSerializer<
		STORAGE_ALIGNMENT,
		VALUE_ALIGNMENT,
		MAX_VALUE_ALIGNMENT,
		MAX_CAPACITY,
		BorrowedStore,
	>
{
	/// Alignment of output buffer
	pub const STORAGE_ALIGNMENT: usize = STORAGE_ALIGNMENT;

	/// Typical alignment of values being serialized
	pub const VALUE_ALIGNMENT: usize = VALUE_ALIGNMENT;

	/// Maximum alignment of values being serialized
	pub const MAX_VALUE_ALIGNMENT: usize = MAX_VALUE_ALIGNMENT;

	/// Maximum capacity of output buffer.
	pub const MAX_CAPACITY: usize = MAX_CAPACITY;
}

impl<
		const STORAGE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize,
		BorrowedStore: BorrowMut<AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT, MAX_CAPACITY>>,
	> PureCopySerializer
	for AlignedSerializer<
		STORAGE_ALIGNMENT,
		VALUE_ALIGNMENT,
		MAX_VALUE_ALIGNMENT,
		MAX_CAPACITY,
		BorrowedStore,
	>
{
}

impl_pure_copy_serializer!(
	AlignedSerializer<
		const STORAGE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize;
		BorrowedStore,
	>
	where BorrowedStore: BorrowMut<AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT, MAX_CAPACITY>>,
);

impl<
		const STORAGE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize,
		BorrowedStore: BorrowMut<AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT, MAX_CAPACITY>>,
	> SerializerStorage
	for AlignedSerializer<
		STORAGE_ALIGNMENT,
		VALUE_ALIGNMENT,
		MAX_VALUE_ALIGNMENT,
		MAX_CAPACITY,
		BorrowedStore,
	>
{
	/// `Storage` which backs this serializer.
	type Store = AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT, MAX_CAPACITY>;

	/// Get immutable ref to `AlignedVec` backing this serializer.
	#[inline]
	fn storage(&self) -> &Self::Store {
		self.storage.borrow()
	}

	/// Get mutable ref to `AlignedVec` backing this serializer.
	#[inline]
	fn storage_mut(&mut self) -> &mut Self::Store {
		self.storage.borrow_mut()
	}
}

impl<
		const STORAGE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize,
	>
	InstantiableSerializer<
		AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT, MAX_CAPACITY>,
	>
	for AlignedSerializer<
		STORAGE_ALIGNMENT,
		VALUE_ALIGNMENT,
		MAX_VALUE_ALIGNMENT,
		MAX_CAPACITY,
		AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT, MAX_CAPACITY>,
	>
{
	/// Create new `AlignedSerializer` with no memory pre-allocated.
	///
	/// If you know, or can estimate, the amount of buffer space that's going to
	/// be needed in advance, allocating upfront with `with_capacity` can
	/// dramatically improve performance vs using `new`.
	#[inline]
	fn new() -> Self {
		Self {
			storage: AlignedVec::new(),
		}
	}

	/// Create new `AlignedSerializer` with buffer pre-allocated with capacity of
	/// at least `capacity` bytes.
	///
	/// `capacity` will be rounded up to a multiple of `MAX_VALUE_ALIGNMENT`.
	///
	/// # Panics
	///
	/// Panics if `capacity` exceeds `MAX_CAPACITY`.
	fn with_capacity(capacity: usize) -> Self {
		// `AlignedVec::with_capacity()` ensures capacity is `< MAX_CAPACITY`
		// and rounds up capacity to a multiple of `MAX_VALUE_ALIGNMENT`
		Self {
			storage: AlignedVec::with_capacity(capacity),
		}
	}
}

impl<
		const STORAGE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize,
		BorrowedStore: BorrowMut<AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT, MAX_CAPACITY>>,
	> BorrowingSerializer<BorrowedStore>
	for AlignedSerializer<
		STORAGE_ALIGNMENT,
		VALUE_ALIGNMENT,
		MAX_VALUE_ALIGNMENT,
		MAX_CAPACITY,
		BorrowedStore,
	>
{
	/// Create new `AlignedSerializer` from an existing `BorrowMut<AlignedVec>`.
	fn from_storage(storage: BorrowedStore) -> Self {
		Self { storage }
	}

	/// Consume Serializer and return the output buffer as a
	/// `BorrowMut<AlignedVec>`.
	fn into_storage(self) -> BorrowedStore {
		self.storage
	}
}
