use std::borrow::BorrowMut;

use crate::{
	pos::{PosMapping, TrackingAddr},
	storage::{AlignedVec, Storage},
	CompleteSerializerTrait, PosTrackingSerializer, PtrSerializer, PtrsRecord, Serialize, Serializer,
	WritableSerializer,
};

/// Serializer that produces a buffer which is a complete valid representation
/// of the input, which can be cast to a `&T` without any deserialization.
///
/// See `AlignedStorage` for an explanation of the const parameters.
pub struct CompleteSerializer<
	const STORAGE_ALIGNMENT: usize,
	const VALUE_ALIGNMENT: usize,
	const MAX_VALUE_ALIGNMENT: usize,
	const MAX_CAPACITY: usize,
	BorrowedStorage: BorrowMut<AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT, MAX_CAPACITY>>,
> {
	storage: BorrowedStorage,
	pos_mapping: PosMapping,
	ptrs_record: PtrsRecord,
}

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize>
	CompleteSerializer<SA, VA, MVA, MAX, AlignedVec<SA, VA, MVA, MAX>>
{
	/// Create new `CompleteSerializer` with no memory pre-allocated.
	///
	/// If you know, or can estimate, the amount of buffer space that's going to
	/// be needed in advance, allocating upfront with `with_capacity` can
	/// dramatically improve performance vs using `new`.
	#[inline]
	pub fn new() -> Self {
		Self {
			storage: AlignedVec::new(),
			pos_mapping: PosMapping::dummy(),
			ptrs_record: PtrsRecord::new(),
		}
	}

	/// Create new `CompleteSerializer` with buffer pre-allocated with capacity of
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
			ptrs_record: PtrsRecord::new(),
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

	/// Create new `CompleteSerializer` from an existing `BorrowMut<AlignedVec>`.
	pub fn from_storage(storage: BorrowedStorage) -> Self {
		Self {
			storage,
			pos_mapping: PosMapping::dummy(),
			ptrs_record: PtrsRecord::new(),
		}
	}
}

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize, BorrowedStorage>
	Serializer for CompleteSerializer<SA, VA, MVA, MAX, BorrowedStorage>
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

	#[inline]
	unsafe fn write<T>(&mut self, value: &T, addr: usize) {
		// Delegate to `WritableSerializer`'s implementation
		WritableSerializer::do_write(self, value, addr);
	}

	#[inline]
	fn write_correction<W: FnOnce(&mut Self)>(&mut self, write: W) {
		// Delegate to `CompleteSerializerTrait`'s implementation
		CompleteSerializerTrait::do_write_correction(self, write);
	}

	#[inline]
	fn finalize(self) -> Self::BorrowedStorage {
		// Delegate to `CompleteSerializerTrait`'s implementation
		CompleteSerializerTrait::do_finalize(self)
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
	PosTrackingSerializer for CompleteSerializer<SA, VA, MVA, MAX, BorrowedStorage>
where BorrowedStorage: BorrowMut<AlignedVec<SA, VA, MVA, MAX>>
{
	/// Get current position mapping.
	#[inline]
	fn pos_mapping(&self) -> &PosMapping {
		&self.pos_mapping
	}

	/// Set current position mapping.
	#[inline]
	fn set_pos_mapping(&mut self, pos_mapping: PosMapping) {
		self.pos_mapping = pos_mapping;
	}
}

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize, BorrowedStorage>
	PtrSerializer for CompleteSerializer<SA, VA, MVA, MAX, BorrowedStorage>
where BorrowedStorage: BorrowMut<AlignedVec<SA, VA, MVA, MAX>>
{
	#[inline]
	unsafe fn write_ptr(&mut self, ptr_pos: usize, target_pos: usize) {
		// Delegate to `CompleteSerializerTrait`'s implementation
		CompleteSerializerTrait::do_write_ptr(self, ptr_pos, target_pos);
	}
}

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize, BorrowedStorage>
	WritableSerializer for CompleteSerializer<SA, VA, MVA, MAX, BorrowedStorage>
where BorrowedStorage: BorrowMut<AlignedVec<SA, VA, MVA, MAX>>
{
}

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize, BorrowedStorage>
	CompleteSerializerTrait for CompleteSerializer<SA, VA, MVA, MAX, BorrowedStorage>
where BorrowedStorage: BorrowMut<AlignedVec<SA, VA, MVA, MAX>>
{
	#[inline]
	fn ptrs_record(&self) -> &PtrsRecord {
		&self.ptrs_record
	}

	#[inline]
	fn ptrs_record_mut(&mut self) -> &mut PtrsRecord {
		&mut self.ptrs_record
	}
}
