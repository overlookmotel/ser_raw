use std::mem;

use crate::{
	ser_traits::{PosTracking, Writable},
	storage::ContiguousStorage,
	util::is_aligned_to,
};

/// Trait for serializers that produce a buffer which is a complete valid
/// representation of the input, which can be cast to a `&T` without any
/// deserialization.
pub trait Complete: PosTracking + Writable
where Self::Storage: ContiguousStorage
{
	// Get reference to record of pointers written.
	fn ptrs(&self) -> &Ptrs;

	// Get mutable reference to record of pointers written.
	fn ptrs_mut(&mut self) -> &mut Ptrs;

	#[inline]
	unsafe fn do_write<T>(&mut self, value: &T, addr: usize) {
		let pos = self.pos_mapping().pos_for_addr(addr);
		self.storage_mut().write(value, pos);
	}

	#[inline]
	fn do_write_correction<W: FnOnce(&mut Self)>(&mut self, write: W) {
		write(self);
	}

	/// Overwrite pointer.
	///
	/// # Safety
	///
	/// * `ptr_pos` and `target_pos` must both sit within bounds of output.
	/// * `target_pos` must be location of a valid value for the type being
	///   pointed to.
	/// * `ptr_pos` must be aligned for a pointer.
	#[inline]
	unsafe fn do_write_ptr(&mut self, ptr_pos: usize, target_pos: usize) {
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
		let ptrs = self.ptrs_mut();
		if storage_addr != ptrs.current.addr() {
			// Storage has moved. Create a new pointer group for new storage address.
			// TODO: Move this to a separate function marked `#[cold]`
			if ptrs.current.is_empty() {
				ptrs.current.set_addr(storage_addr);
			} else {
				let new_ptr_group = PtrGroup::new(storage_addr);
				let old_ptr_group = mem::replace(&mut ptrs.current, new_ptr_group);
				ptrs.past.push(old_ptr_group);
			}
		}
		ptrs.current.push_pos(ptr_pos);
	}

	/// Finalize the serialized output, updating any pointers which have been made
	/// invalid because storage moved since the pointers were written.
	///
	/// After this, the serializer cannot be used any further, so this method
	/// consumes it and returns the underlying `BorrowMut<Storage>`.
	fn do_finalize(mut self) -> Self::BorrowedStorage {
		let storage_ptr = self.storage_mut().as_mut_ptr();

		let ptrs = self.ptrs_mut();

		// Safe if all pointers have been recorded accurately
		unsafe {
			if ptrs.current.addr() != storage_ptr as usize && !ptrs.current.is_empty() {
				ptrs.current.correct_ptrs(storage_ptr);
			}

			for ptr_group in &ptrs.past {
				if ptr_group.addr() != storage_ptr as usize {
					ptr_group.correct_ptrs(storage_ptr);
				}
			}
		}

		self.into_storage()
	}
}

// TODO: If also provided a `Storage` with fixed capacity which can never move,
// recording pointers for later correction could be skipped as they'll always be
// accurate when they're written.

/// A record of pointers written to storage which may require correction if
/// storage grows during serializtion and its memory address changes.
///
/// `current` is the group of of pointers currently in use.
/// `past` is previous groups.
/// Each time a change in memory address for the storage buffer is detected,
/// `current` is added to `past` and a fresh `current` is created.
pub struct Ptrs {
	pub current: PtrGroup,
	pub past: Vec<PtrGroup>,
}

impl Ptrs {
	pub fn new() -> Ptrs {
		Ptrs {
			current: PtrGroup::dummy(),
			past: Vec::new(),
		}
	}
}

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
	pub unsafe fn correct_ptrs(&self, storage_ptr: *mut u8) {
		// These pointers were written when start of storage was at
		// `ptr_group.storage_addr`. Now it's at `storage_addr`.
		// Shift pointers' target addresses forward or backwards as required so they
		// point to targets' current memory addresses.
		// Using `wrapping_*` for correct maths for all possible old + new addresses,
		// regardless of whether new addr is less than or greater than old addr.
		// No need to cast to `isize` to handle negative shift.
		// e.g. `old = 4`, `new = 10` -> `shift_by = 6` -> each ptr has 6 added.
		let shift_by = (storage_ptr as usize).wrapping_sub(self.storage_addr);
		for ptr_pos in &self.ptr_positions {
			// TODO: Use `storage.read()` and `storage.write()` instead of this
			let ptr = storage_ptr.add(*ptr_pos) as *mut usize;
			*ptr = (*ptr).wrapping_add(shift_by);
		}
	}
}
