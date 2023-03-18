use std::{borrow::BorrowMut, mem};

use crate::{
	impl_ptr_serializer,
	pos::PosMapping,
	storage::{AlignedVec, ContiguousStorage, Storage},
	util::is_aligned_to,
	BorrowingSerializer, InstantiableSerializer, PosTrackingSerializer, PtrSerializer, Serializer,
	SerializerStorage, SerializerWrite,
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
	current_ptr_group: PtrGroup,
	ptr_groups: Vec<PtrGroup>,
}

// TODO: If also provided a `Storage` with fixed capacity which can never move,
// recording pointers for later correction could be skipped as they'll always be
// accurate when they're written.

/// A group of pointers which were written to storage when the memory address of
/// the storage was `storage_addr`.
/// Used for correcting pointers if the storage grows during serialization and
/// its memory address changes.
// TODO: Use `u32` for ptr positions if `MAX_CAPACITY` is less than `u32::MAX`
pub struct PtrGroup {
	/// Memory address of the storage at time pointers in this group were created
	storage_addr: usize,
	/// Positions of pointers in storage (relative to start of storage)
	ptr_positions: Vec<usize>,
}

impl PtrGroup {
	#[inline]
	pub fn new(storage_addr: usize) -> Self {
		Self {
			storage_addr,
			// TODO: Maybe replace with `with_capacity(32)` or similar to avoid repeated growing?
			ptr_positions: Vec::new(),
		}
	}

	#[inline]
	pub fn dummy() -> Self {
		Self::new(0)
	}

	#[inline]
	pub fn is_empty(&self) -> bool {
		self.ptr_positions.len() == 0
	}

	#[inline]
	pub fn addr(&self) -> usize {
		self.storage_addr
	}

	#[inline]
	pub fn set_addr(&mut self, storage_addr: usize) {
		self.storage_addr = storage_addr;
	}

	#[inline]
	pub fn push_pos(&mut self, pos: usize) {
		self.ptr_positions.push(pos);
	}

	/// Correct pointers in storage.
	///
	/// # Safety
	///
	/// All `ptr_positions` must be within the bounds of the `Storage` pointed to
	/// by `storage_ptr`.
	pub unsafe fn correct_ptrs(self, storage_ptr: *mut u8) {
		// These pointers were written when start of storage was at
		// `ptr_group.storage_addr`. Now it's at `storage_addr`.
		// Shift pointers' target addresses forward or backwards as required so they
		// point to targets' current memory addresses.
		// Using `wrapping_*` for correct maths for all possible old + new addresses,
		// regardless of whether new addr is less than or greater than old addr.
		// No need to cast to `isize` to handle negative shift.
		// e.g. `old = 4`, `new = 10` -> `shift_by = 6` -> each ptr has 6 added.
		let shift_by = (storage_ptr as usize).wrapping_sub(self.storage_addr);
		for ptr_pos in self.ptr_positions {
			// TODO: Use `storage.read()` and `storage.write()` instead of this
			let ptr = storage_ptr.add(ptr_pos) as *mut usize;
			*ptr = (*ptr).wrapping_add(shift_by);
		}
	}
}

// Expose const params as associated consts - `Self::STORAGE_ALIGNMENT` etc.
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

	/// Finalize the serialized output, updating any pointers which have been made
	/// invalid because storage moved since the pointers were written.
	///
	/// After this, the serializer cannot be used any further, so this method
	/// consumes it and returns the underlying `BorrowMut<Storage>`.
	pub fn finalize(mut self) -> BorrowedStorage {
		let storage_ptr = self.storage_mut().as_mut_ptr();

		// Safe if all pointers have been recorded accurately
		unsafe {
			if self.current_ptr_group.addr() != storage_ptr as usize && !self.current_ptr_group.is_empty()
			{
				self.current_ptr_group.correct_ptrs(storage_ptr);
			}

			for ptr_group in self.ptr_groups {
				if ptr_group.addr() != storage_ptr as usize {
					ptr_group.correct_ptrs(storage_ptr);
				}
			}
		}

		self.storage
	}
}

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize, BorrowedStorage>
	PtrSerializer for CompleteSerializer<SA, VA, MVA, MAX, BorrowedStorage>
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

		// Write pointer to storage (pointing to real address of target)
		let storage_addr = self.storage().as_ptr() as usize;
		let target_addr = storage_addr + target_pos;
		self.storage_mut().write(&target_addr, ptr_pos);

		// Record position of this pointer in storage so can be adjusted later if
		// storage grows and so moves
		if storage_addr != self.current_ptr_group.addr() {
			// Storage has moved. Create a new pointer group for new storage address.
			// TODO: Move this to a separate function marked `#[cold]`
			if self.current_ptr_group.is_empty() {
				self.current_ptr_group.set_addr(storage_addr);
			} else {
				let new_ptr_group = PtrGroup::new(storage_addr);
				let old_ptr_group = mem::replace(&mut self.current_ptr_group, new_ptr_group);
				self.ptr_groups.push(old_ptr_group);
			}
		}
		self.current_ptr_group.push_pos(ptr_pos);
	}
}

impl_ptr_serializer!(
	CompleteSerializer<
		const SA: usize, const VA: usize, const MVA: usize, const MAX: usize; BorrowedStorage
	>
	where BorrowedStorage: BorrowMut<AlignedVec<SA, VA, MVA, MAX>>,
);

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize, BorrowedStorage>
	PosTrackingSerializer for CompleteSerializer<SA, VA, MVA, MAX, BorrowedStorage>
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

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize, BorrowedStorage>
	SerializerStorage for CompleteSerializer<SA, VA, MVA, MAX, BorrowedStorage>
where BorrowedStorage: BorrowMut<AlignedVec<SA, VA, MVA, MAX>>
{
	/// `Storage` which backs this serializer.
	type Storage = AlignedVec<SA, VA, MVA, MAX>;
	type BorrowedStorage = BorrowedStorage;

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

	#[inline]
	fn into_storage(self) -> BorrowedStorage {
		self.storage
	}
}

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize, BorrowedStorage>
	SerializerWrite for CompleteSerializer<SA, VA, MVA, MAX, BorrowedStorage>
where BorrowedStorage: BorrowMut<AlignedVec<SA, VA, MVA, MAX>>
{
	#[inline]
	unsafe fn write<T>(&mut self, value: &T, addr: usize) {
		let pos = self.pos_mapping().pos_for_addr(addr);
		self.storage_mut().write(value, pos);
	}

	#[inline]
	fn write_correction<W: FnOnce(&mut Self)>(&mut self, write: W) {
		write(self);
	}
}

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize> InstantiableSerializer
	for CompleteSerializer<SA, VA, MVA, MAX, AlignedVec<SA, VA, MVA, MAX>>
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
			current_ptr_group: PtrGroup::dummy(),
			ptr_groups: Vec::new(),
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
			current_ptr_group: PtrGroup::dummy(),
			ptr_groups: Vec::new(),
		}
	}
}

impl<const SA: usize, const VA: usize, const MVA: usize, const MAX: usize, BorrowedStorage>
	BorrowingSerializer for CompleteSerializer<SA, VA, MVA, MAX, BorrowedStorage>
where BorrowedStorage: BorrowMut<AlignedVec<SA, VA, MVA, MAX>>
{
	/// Create new `AlignedSerializer` from an existing `BorrowMut<AlignedVec>`.
	fn from_storage(storage: BorrowedStorage) -> Self {
		Self {
			storage,
			pos_mapping: PosMapping::dummy(),
			current_ptr_group: PtrGroup::dummy(),
			ptr_groups: Vec::new(),
		}
	}
}
