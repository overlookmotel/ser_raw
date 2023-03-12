use std::borrow::BorrowMut;

use crate::{
	storage::{AlignedVec, Storage},
	BorrowingSerializer, InstantiableSerializer, Serializer,
};

/// Serializer that ensures values are correctly aligned in output buffer.
///
/// # Const parameters
///
/// `STORAGE_ALIGNMENT` is the alignment of the output buffer.
///
/// `MAX_VALUE_ALIGNMENT` is maximum alignment of types which will be
/// serialized. Types with alignment greater than `MAX_VALUE_ALIGNMENT` cannot
/// be serialized with this serializer.
///
/// `MAX_CAPACITY` is maximum capacity of storage. Cannot be 0 and cannot be
/// greater than `isize::MAX + 1 - STORAGE_ALIGNMENT`. Must be a multiple of
/// `MAX_VALUE_ALIGNMENT`.
///
/// `VALUE_ALIGNMENT` is minimum alignment all allocated values will have in
/// output buffer. Types with alignment higher than `VALUE_ALIGNMENT` will have
/// padding inserted before them if required. Types with alignment lower than
/// `VALUE_ALIGNMENT` will have padding inserted after to leave the buffer
/// aligned on `VALUE_ALIGNMENT` for the next insertion.
///
/// This doesn't affect the "legality" of the output, but if most allocated
/// types being serialized have the same alignment, setting `VALUE_ALIGNMENT` to
/// that alignment may significantly improve performance, as alignment
/// calculations can be skipped when serializing those types.
///
/// NB: The word "allocated" in "allocated types" is key here. `ser_raw` deals
/// in allocations, not individual types. So this means that only types which
/// are pointed to by a `Box<T>` or `Vec<T>` count as "allocated types"
/// for the purposes of calculating an optimal value for `VALUE_ALIGNMENT`.
///
/// e.g. If all (or almost all) types contain pointers (`Box`, `Vec` etc),
/// setting `VALUE_ALIGNMENT = std::mem::size_of::<usize>()`
/// will be the best value for fast serialization.
///
/// The higher `VALUE_ALIGNMENT` is, the more padding bytes will end up in
/// output, potentially increasing output size, depending on the types being
/// serialized.
pub struct AlignedSerializer<
	BorrowedStore: BorrowMut<AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT, MAX_CAPACITY>>,
	const STORAGE_ALIGNMENT: usize,
	const VALUE_ALIGNMENT: usize,
	const MAX_VALUE_ALIGNMENT: usize,
	const MAX_CAPACITY: usize,
> {
	storage: BorrowedStore,
}

impl<
		BorrowedStore: BorrowMut<AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT, MAX_CAPACITY>>,
		const STORAGE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize,
	>
	AlignedSerializer<
		BorrowedStore,
		STORAGE_ALIGNMENT,
		VALUE_ALIGNMENT,
		MAX_VALUE_ALIGNMENT,
		MAX_CAPACITY,
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
		BorrowedStore: BorrowMut<AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT, MAX_CAPACITY>>,
		const STORAGE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize,
	> Serializer
	for AlignedSerializer<
		BorrowedStore,
		STORAGE_ALIGNMENT,
		VALUE_ALIGNMENT,
		MAX_VALUE_ALIGNMENT,
		MAX_CAPACITY,
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
		AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT, MAX_CAPACITY>,
		STORAGE_ALIGNMENT,
		VALUE_ALIGNMENT,
		MAX_VALUE_ALIGNMENT,
		MAX_CAPACITY,
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
		BorrowedStore: BorrowMut<AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT, MAX_CAPACITY>>,
		const STORAGE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize,
	> BorrowingSerializer<BorrowedStore>
	for AlignedSerializer<
		BorrowedStore,
		STORAGE_ALIGNMENT,
		VALUE_ALIGNMENT,
		MAX_VALUE_ALIGNMENT,
		MAX_CAPACITY,
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
