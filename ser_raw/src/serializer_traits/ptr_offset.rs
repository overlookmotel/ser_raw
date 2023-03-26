use std::mem;

use crate::{ser_traits::PosTracking, storage::ContiguousStorage, util::is_aligned_to};

/// Trait for serializers which overwrite pointers in output with position
/// offsets relative to start of output.
///
/// Used by `PtrOffsetSerializer` serializer, provided by this crate.
pub trait PtrOffset: PosTracking
where Self::Storage: ContiguousStorage
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
	unsafe fn do_write_ptr(&mut self, ptr_pos: usize, target_pos: usize) {
		// Cannot fully check validity of `target_pos` because its type isn't known
		debug_assert!(ptr_pos <= self.capacity() - mem::size_of::<usize>());
		debug_assert!(is_aligned_to(ptr_pos, mem::align_of::<usize>()));
		debug_assert!(target_pos <= self.capacity());

		self.storage_mut().write(ptr_pos, &target_pos);
	}
}
