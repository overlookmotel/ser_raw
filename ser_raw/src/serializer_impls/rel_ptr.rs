use std::{borrow::BorrowMut, mem};

use crate::{
	impl_ptr_serializer,
	pos::PosMapping,
	storage::{AlignedVec, ContiguousStorage, Storage},
	util::is_aligned_to,
	BorrowingSerializer, InstantiableSerializer, PosTrackingSerializer, PtrSerializer, Serializer,
	SerializerStorage,
};

/// Serializer that ensures values are correctly aligned in output buffer
/// and overwrites pointers in output with pointers relative to the start of the
/// buffer.
///
/// See `AlignedStorage` for an explanation of the const parameters.
pub struct AlignedRelPtrSerializer<
	const STORAGE_ALIGNMENT: usize,
	const VALUE_ALIGNMENT: usize,
	const MAX_VALUE_ALIGNMENT: usize,
	const MAX_CAPACITY: usize,
	BorrowedStore: BorrowMut<AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT, MAX_CAPACITY>>,
> {
	storage: BorrowedStore,
	pos_mapping: PosMapping,
}

// Expose const params as associated consts - `Self::STORAGE_ALIGNMENT` etc.
impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize, BorrowedStore>
	AlignedRelPtrSerializer<SA, VA, MVA, MAX, BorrowedStore>
where BorrowedStore: BorrowMut<AlignedVec<SA, VA, MVA, MAX>>
{
	/// Alignment of output buffer
	pub const STORAGE_ALIGNMENT: usize = SA;

	/// Typical alignment of values being serialized
	pub const VALUE_ALIGNMENT: usize = VA;

	/// Maximum alignment of values being serialized
	pub const MAX_VALUE_ALIGNMENT: usize = MVA;

	/// Maximum capacity of output buffer.
	pub const MAX_CAPACITY: usize = MAX;
}

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize, BorrowedStore>
	PtrSerializer for AlignedRelPtrSerializer<SA, VA, MVA, MAX, BorrowedStore>
where BorrowedStore: BorrowMut<AlignedVec<SA, VA, MVA, MAX>>
{
	/// Overwrite pointer.
	///
	/// # Safety
	///
	/// * `ptr_pos` and `target_pos` must both sit within bounds of output.
	/// * `target_pos` must be location of a valid value for the type being
	///   pointed to.
	/// * `ptr_pos` must be aligned for a pointer.
	#[inline]
	unsafe fn write_ptr(&mut self, ptr_pos: usize, target_pos: usize) {
		// Cannot fully check validity of `target_pos` because its type isn't known
		debug_assert!(ptr_pos <= self.capacity() - mem::size_of::<usize>());
		debug_assert!(is_aligned_to(ptr_pos, mem::align_of::<usize>()));
		debug_assert!(target_pos <= self.capacity());

		self.storage_mut().write(&target_pos, ptr_pos)
	}
}

impl_ptr_serializer!(
	AlignedRelPtrSerializer<
		const SA: usize, const VA: usize, const MVA: usize, const MAX: usize; BorrowedStore
	>
	where BorrowedStore: BorrowMut<AlignedVec<SA, VA, MVA, MAX>>,
);

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize, BorrowedStore>
	PosTrackingSerializer for AlignedRelPtrSerializer<SA, VA, MVA, MAX, BorrowedStore>
where BorrowedStore: BorrowMut<AlignedVec<SA, VA, MVA, MAX>>
{
	/// Get current position mapping
	fn pos_mapping(&self) -> &PosMapping {
		&self.pos_mapping
	}

	/// Set current position mapping
	fn set_pos_mapping(&mut self, pos_mapping: PosMapping) {
		self.pos_mapping = pos_mapping;
	}
}

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize, BorrowedStore>
	SerializerStorage for AlignedRelPtrSerializer<SA, VA, MVA, MAX, BorrowedStore>
where BorrowedStore: BorrowMut<AlignedVec<SA, VA, MVA, MAX>>
{
	/// `Storage` which backs this serializer.
	type Store = AlignedVec<SA, VA, MVA, MAX>;

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

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize>
	InstantiableSerializer<AlignedVec<SA, VA, MVA, MAX>>
	for AlignedRelPtrSerializer<SA, VA, MVA, MAX, AlignedVec<SA, VA, MVA, MAX>>
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
			pos_mapping: PosMapping::dummy(),
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
			pos_mapping: PosMapping::dummy(),
		}
	}
}

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize, BorrowedStore>
	BorrowingSerializer<BorrowedStore> for AlignedRelPtrSerializer<SA, VA, MVA, MAX, BorrowedStore>
where BorrowedStore: BorrowMut<AlignedVec<SA, VA, MVA, MAX>>
{
	/// Create new `AlignedSerializer` from an existing `BorrowMut<AlignedVec>`.
	fn from_storage(storage: BorrowedStore) -> Self {
		Self {
			storage,
			pos_mapping: PosMapping::dummy(),
		}
	}

	/// Consume Serializer and return the output buffer as a
	/// `BorrowMut<AlignedVec>`.
	fn into_storage(self) -> BorrowedStore {
		self.storage
	}
}
