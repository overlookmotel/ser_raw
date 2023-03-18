use std::{borrow::BorrowMut, mem};

use crate::{
	pos::{PosMapping, TrackingAddr},
	storage::{AlignedVec, ContiguousStorage, Storage},
	util::is_aligned_to,
	PosTrackingSerializer, PtrSerializer, Serialize, Serializer,
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
	BorrowedStorage: BorrowMut<AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT, MAX_CAPACITY>>,
> {
	storage: BorrowedStorage,
	pos_mapping: PosMapping,
}

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize>
	AlignedRelPtrSerializer<SA, VA, MVA, MAX, AlignedVec<SA, VA, MVA, MAX>>
{
	/// Create new `AlignedSerializer` with no memory pre-allocated.
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

	/// Create new `AlignedSerializer` with buffer pre-allocated with capacity of
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
			pos_mapping: PosMapping::dummy(),
		}
	}
}

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize, BorrowedStorage>
	AlignedRelPtrSerializer<SA, VA, MVA, MAX, BorrowedStorage>
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

	/// Create new `AlignedRelPtrSerializer` from an existing
	/// `BorrowMut<AlignedVec>`.
	pub fn from_storage(storage: BorrowedStorage) -> Self {
		Self {
			storage,
			pos_mapping: PosMapping::dummy(),
		}
	}
}

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize, BorrowedStorage>
	Serializer for AlignedRelPtrSerializer<SA, VA, MVA, MAX, BorrowedStorage>
where BorrowedStorage: BorrowMut<AlignedVec<SA, VA, MVA, MAX>>
{
	/// `Storage` which backs this serializer.
	type Storage = AlignedVec<SA, VA, MVA, MAX>;
	type BorrowedStorage = BorrowedStorage;

	/// Pointer serializers do record pointers, so need a functional `Addr`.
	type Addr = TrackingAddr;

	fn serialize_value<T: Serialize<Self>>(&mut self, value: &T) {
		// Delegate to `PtrSerializer`'s implementation
		PtrSerializer::do_serialize_value(self, value);
	}

	// Skip recording position mapping here because no further processing of the
	// slice, but still write pointer
	#[inline]
	fn push_slice<T>(&mut self, slice: &[T], ptr_addr: Self::Addr) {
		// Delegate to `PtrSerializer`'s implementation
		PtrSerializer::do_push_slice(self, slice, ptr_addr);
	}

	#[inline]
	fn push_and_process_slice<T, P: FnOnce(&mut Self)>(
		&mut self,
		slice: &[T],
		ptr_addr: Self::Addr,
		process: P,
	) {
		// Delegate to `PtrSerializer`'s implementation
		PtrSerializer::do_push_and_process_slice(self, slice, ptr_addr, process);
	}

	/// Get immutable ref to `AlignedVec` backing this serializer.
	#[inline]
	fn storage(&self) -> &Self::Storage {
		self.storage.borrow()
	}

	/// Get mutable ref to `AlignedVec` backing this serializer.
	#[inline]
	fn storage_mut(&mut self) -> &mut Self::Storage {
		self.storage.borrow_mut()
	}

	/// Consume Serializer and return the backing storage as a
	/// `BorrowMut<Storage>`.
	#[inline]
	fn into_storage(self) -> BorrowedStorage {
		self.storage
	}
}

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize, BorrowedStorage>
	PtrSerializer for AlignedRelPtrSerializer<SA, VA, MVA, MAX, BorrowedStorage>
where BorrowedStorage: BorrowMut<AlignedVec<SA, VA, MVA, MAX>>
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

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize, BorrowedStorage>
	PosTrackingSerializer for AlignedRelPtrSerializer<SA, VA, MVA, MAX, BorrowedStorage>
where BorrowedStorage: BorrowMut<AlignedVec<SA, VA, MVA, MAX>>
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
