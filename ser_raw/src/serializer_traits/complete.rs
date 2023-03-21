use std::mem;

use crate::{
	pos::{PtrGroup, Ptrs},
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
			new_ptr_group(ptrs, storage_addr);
		}
		ptrs.current.push_pos(ptr_pos);

		// Separate function to guide inlining and branch prediction.
		// This should rarely be called, as storage growth is an occasional event.
		#[cold]
		fn new_ptr_group(ptrs: &mut Ptrs, storage_addr: usize) {
			if ptrs.current.is_empty() {
				ptrs.current.set_addr(storage_addr);
			} else {
				let new_ptr_group = PtrGroup::new(storage_addr);
				let old_ptr_group = mem::replace(&mut ptrs.current, new_ptr_group);
				ptrs.past.push(old_ptr_group);
			}
		}
	}

	/// Finalize the serialized output, updating any pointers which have been made
	/// invalid because storage moved since the pointers were written.
	///
	/// After this, the serializer cannot be used any further, so this method
	/// consumes it and returns the underlying `BorrowMut<Storage>`.
	#[inline]
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
