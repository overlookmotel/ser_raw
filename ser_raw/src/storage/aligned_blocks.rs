use std::{mem, num::NonZeroUsize};

use super::{aligned::AlignmentCheck, AlignedBytes, AlignedStorage, Storage};
use crate::util::{align_up_to, is_aligned_to};

const PTR_SIZE: usize = mem::size_of::<usize>();
const DEFAULT_STORAGE_ALIGNMENT: usize = 16;
const DEFAULT_VALUE_ALIGNMENT: usize = PTR_SIZE;
const DEFAULT_MAX_CAPACITY: usize = (isize::MAX as usize) + 1;

// Make this an associated const
const MAX_BLOCK_COUNT: usize = PTR_SIZE * 8;

/// Aligned storage which allocates memory in a series of blocks.
///
/// Each block is allocated as required, and then never grows, so all data in
/// the storage maintains a stable memory address.
///
/// Total capacity is always a power of 2, and grows by doubling total capacity
/// at minimum. Capacity can grow in larger jumps if a value larger than the
/// current capacity needs to be stored.
///
/// Maximum capacity is `isize::MAX + 1`. Maximum initial capacity for
/// [`with_capacity`] is half of that, but the storage can then grow once up to
/// the maximum.
///
/// Any capacity up to the maximum can be accomodated by 64 blocks or less
/// (32 blocks on 32-bit systems).
///
/// The number of blocks used will be less if:
///
/// * The storage is not filled up to maximum capacity.
/// * [`AlignedBlocks`] is initialized with [`with_capacity`].
///
/// # Implementation details
///
/// <details><summary>Click to view</summary>
///
/// Growth strategy ensures total capacity is always a power of 2.
///
/// The purpose of this strategy is to allow fast translation from a position in
/// the storage overall to the block that the data resides in, and position in
/// that block.
///
/// 64 blocks can fulfill any storage capacity up to maximum `isize::MAX + 1`
/// (or 32 blocks on 32-bit systems). The maximum number of blocks required is
/// lower if the [`AlignedBlocks`] is initialized with a larger starting
/// capacity.
///
/// A position's `magnitude` is the number of leading zeros it has in its binary
/// representation. Obtaining `magnitude` is a single processor instruction.
///
/// Position is a `usize`, so `magnitude` is between 0 and 64 (inclusive),
/// (or 0-32 on 32-bit systems).
/// So magnitude has 65 possible values (33 on 32-bit systems).
/// But position 0 (`magnitude` 64) is handled as a special case.
/// NB perhaps "magnitude" is a misleading name, as small numbers have high
/// "magnitude", and large numbers have low "magnitude".
///
/// `block_indexes` maps from `magnitude` to the the block index
/// (`block_indexes[magnitude]`). `block_indexes` is only 64 bytes, so can be
/// implemented as a statically-sized contiguous `[u8; 64]`, which is cheap to
/// index into.
///
/// The starting position of each block is recorded in `block_positions` so
/// obtaining the offset from start of block for this position is just
/// `pos - block_positions[block_index]`. Again, cheap.
///
/// A valid position can never have `magnitude` of 0, because that would
/// require a position > `isize::MAX`, which is always out of bounds.
/// So value in `block_indexes[0]` is redundant. We do not try to exploit this,
/// to reduce size of `block_indexes` to 63, because would require an
/// extra `- 1` operation on all lookups.
///
/// </details>
///
/// [`with_capacity`]: AlignedBlocks::with_capacity
pub struct AlignedBlocks<
	const STORAGE_ALIGNMENT: usize = DEFAULT_STORAGE_ALIGNMENT,
	const MAX_VALUE_ALIGNMENT: usize = STORAGE_ALIGNMENT,
	const VALUE_ALIGNMENT: usize = DEFAULT_VALUE_ALIGNMENT,
	const MAX_CAPACITY: usize = DEFAULT_MAX_CAPACITY,
> {
	/// Total current capacity of storage.
	capacity: usize,
	/// Total used storage.
	len: usize,
	/// Number of blocks (including current block).
	block_count: u8,
	/// Current block which new pushes will add to.
	current_block: AlignedBytes,
	/// Past blocks which are now full.
	blocks: Box<[AlignedBytes]>,
	/// Start position of blocks.
	block_positions: Box<[usize]>,
	/// Mapping from position magnitude to block index.
	// Boxed to avoid size of `AlignedBlocks` exceeding 128 bytes.
	// TODO: Wrap `[u8; 64]` in a `#[repr(align(64))]` type
	// so this always occupies a single cache line?
	block_indexes: Box<[u8; MAX_BLOCK_COUNT]>,
}

impl<
		const STORAGE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize,
	> Storage for AlignedBlocks<STORAGE_ALIGNMENT, MAX_VALUE_ALIGNMENT, VALUE_ALIGNMENT, MAX_CAPACITY>
{
	/// Create new [`AlignedBlocks`] with no pre-allocated capacity.
	///
	/// A first block of memory will be allocated when is first pushed to.
	///
	/// To avoid creating lots of small blocks, it's recommended to use
	/// [`with_capacity`](AlignedBlocks::with_capacity) instead.
	fn new() -> Self {
		// Ensure (at compile time) that const params are valid
		let _ = Self::ASSERT_ALIGNMENTS_VALID;

		// Get max number of blocks that may be required to fulfill any capacity up to
		// maximum (`isize::MAX + 1`).
		// Max blocks has upper bound of 64 when 1st block's size is 1.
		// But a larger initial block size means there can be less cycles of growth,
		// so less blocks are required.
		// On first push, capacity of 1st block will be `MAX_VALUE_ALIGNMENT` or more.
		// NB: `magnitude_for_non_zero(1) + 1 == 64`
		//
		// `capacity` cannot be zero (precondition)
		let max_num_blocks = unsafe { magnitude_for_non_zero(MAX_VALUE_ALIGNMENT) } + 1;

		Self {
			capacity: 0,
			len: 0,
			block_count: 0,
			current_block: AlignedBytes::new(),
			blocks: create_default_boxed_slice::<AlignedBytes>(max_num_blocks),
			block_positions: create_default_boxed_slice::<usize>(max_num_blocks),
			block_indexes: Box::new([0; MAX_BLOCK_COUNT]),
		}
	}

	/// Create new [`AlignedBlocks`] with pre-allocated capacity.
	///
	/// Capacity will be rounded up to a power of 2 with minimum
	/// `MAX_VALUE_ALIGNMENT`.
	///
	/// The larger the initial capacity allocated, the less data will be split
	/// across multiple blocks, which is preferable for performance.
	///
	/// # Panics
	///
	/// Panics if `capacity` exceeds `MAX_CAPACITY` or `(isize::MAX + 1) / 2`.
	fn with_capacity(capacity: usize) -> Self {
		// Ensure (at compile time) that const params are valid
		let _ = Self::ASSERT_ALIGNMENTS_VALID;

		if capacity == 0 {
			return Self::new();
		}

		let capacity = if capacity <= MAX_VALUE_ALIGNMENT {
			// `MAX_VALUE_ALIGNMENT` is always a power of 2
			MAX_VALUE_ALIGNMENT
		} else {
			// Cannot allocate `isize::MAX + 1` in a single allocation due to requirement of
			// `std::alloc::Layout`, so limit first allocation to `(isize::MAX + 1) / 2`.
			assert!(
				capacity <= MAX_CAPACITY && capacity <= DEFAULT_MAX_CAPACITY / 2,
				"Requested capacity exceeds maximum for first allocation"
			);

			// Round up capacity to a power of 2.
			// Any power of 2 larger than `MAX_VALUE_ALIGNMENT` is also a multiple of
			// `MAX_VALUE_ALIGNMENT`.
			// Assertion above ensures overflow in `next_power_of_two()` is not possible.
			// TODO: Is there a faster method if this is a `NonZeroUsize`?
			capacity.next_power_of_two()
		};

		// Above checks satisfy `with_capacity_unchecked`'s requirements
		unsafe { Self::with_capacity_unchecked(capacity) }
	}

	/// Create new [`AlignedBlocks`] with pre-allocated capacity,
	/// without safety checks.
	///
	/// # Safety
	///
	/// * `capacity` cannot be 0.
	/// * `capacity` must be a power of 2.
	/// * `capacity` must be a greater or equal to `MAX_VALUE_ALIGNMENT`.
	/// * `capacity` must be less than or equal to `MAX_CAPACITY`.
	/// * `capacity` must be less than or equal to `(isize::MAX + 1) / 2`.
	unsafe fn with_capacity_unchecked(capacity: usize) -> Self {
		// Ensure (at compile time) that const params are valid
		let _ = Self::ASSERT_ALIGNMENTS_VALID;

		debug_assert!(capacity > 0, "capacity cannot be 0");
		debug_assert!(capacity.is_power_of_two(), "capacity must be a power of 2");
		debug_assert!(
			capacity >= MAX_VALUE_ALIGNMENT,
			"capacity must be >= MAX_VALUE_ALIGNMENT"
		);
		debug_assert!(
			capacity <= MAX_CAPACITY && capacity <= DEFAULT_MAX_CAPACITY / 2,
			"capacity cannot exceed MAX_CAPACITY or (isize::MAX + 1) / 2"
		);

		// Get max number of blocks that may be required to fulfill any capacity up to
		// maximum (`isize::MAX + 1`).
		// Max blocks has upper bound of 64 when 1st block's size is 1.
		// But a larger initial block size means there can be less cycles of growth,
		// so less blocks are required.
		// NB: `magnitude_for_non_zero(1) + 1 == 64`
		//
		// `capacity` cannot be zero (precondition)
		let max_num_blocks = unsafe { magnitude_for_non_zero(capacity) } + 1;

		Self {
			capacity,
			len: 0,
			block_count: 1,
			current_block: AlignedBytes::with_capacity(capacity),
			blocks: create_default_boxed_slice::<AlignedBytes>(max_num_blocks),
			block_positions: create_default_boxed_slice::<usize>(max_num_blocks),
			block_indexes: Box::new([0; MAX_BLOCK_COUNT]),
		}
	}

	/// Returns current capacity of this [`AlignedBlocks`] in bytes.
	#[inline]
	fn capacity(&self) -> usize {
		self.capacity
	}

	/// Returns amount of storage currently used in this [`AlignedBlocks`] in
	/// bytes.
	#[inline]
	fn len(&self) -> usize {
		self.len
	}

	/// Set amount of storage space used (in bytes).
	///
	/// # Safety
	///
	/// `new_len` must be less than or equal to `capacity()`.
	#[inline]
	unsafe fn set_len(&mut self, new_len: usize) {
		debug_assert!(new_len <= self.capacity());

		self.len = new_len;
		// TODO: Set `len` of current block too
	}

	/// Push a slice of values `&T` to storage, without alignment checks.
	///
	/// # Panics
	///
	/// Panics if would require growing storage beyond `MAX_CAPACITY`.
	///
	/// # Safety
	///
	/// This method does *not* ensure 2 invariants relating to alignment:
	///
	/// * `len` must be aligned for the type before push.
	/// * `len` must be aligned to `VALUE_ALIGNMENT` after push.
	///
	/// Caller must uphold these invariants. It is sufficient to:
	///
	/// * call `align_for::<T>()` before and
	/// * call `align_after::<T>()` after.
	// TODO: This is a copy of `AlignedVec`'s method. De-dupe code.
	#[inline]
	unsafe fn push_slice_unaligned<T>(&mut self, slice: &[T]) {
		debug_assert!(is_aligned_to(self.len(), mem::align_of::<T>()));

		// Do nothing if ZST. This function will be compiled down to a no-op for ZSTs.
		if mem::size_of::<T>() == 0 {
			return;
		}

		// Calculating `size` can't overflow as that would imply this is a slice of
		// `usize::MAX + 1` or more bytes, which can't be possible.
		let size = mem::size_of::<T>() * slice.len();
		self.reserve(size);

		// `reserve()` ensures sufficient capacity.
		// `size` is calculated correctly above.
		// Ensuring alignment is a requirment of this method.
		self.push_slice_unchecked(slice, size);
	}

	/// Push a slice of values `&T` to storage, without alignment checks and
	/// without reserving capacity for it.
	///
	/// # Safety
	///
	/// Caller must ensure [`AlignedBlocks`] has sufficient capacity.
	///
	/// `size` must be total size in bytes of `&[T]`.
	/// i.e. `size = mem::size_of::<T>() * slice.len()`.
	///
	/// This method does *not* ensure 2 invariants of storage relating to
	/// alignment:
	///
	/// * that `len` is aligned for the type before push.
	/// * that `len` is aligned to `VALUE_ALIGNMENT` after push.
	///
	/// Caller must uphold these invariants. It is sufficient to:
	///
	/// * call `align_for::<T>()` before and
	/// * call `align_after::<T>()` after.
	#[inline]
	unsafe fn push_slice_unchecked<T>(&mut self, slice: &[T], size: usize) {
		debug_assert!(self.capacity() - self.len() >= size);
		debug_assert_eq!(size, mem::size_of::<T>() * slice.len());
		debug_assert!(is_aligned_to(self.len(), mem::align_of::<T>()));

		// Do nothing if ZST. This function will be compiled down to a no-op for ZSTs.
		if mem::size_of::<T>() == 0 {
			return;
		}

		self.current_block.push_slice_unchecked(slice, size);
	}

	/// Align position in storage to alignment of `T`.
	// TODO: This is a copy of `AlignedVec`'s method. De-dupe code.
	#[inline(always)] // Because this is generally a no-op
	fn align_for<T>(&mut self) {
		// Ensure (at compile time) that `T`'s alignment does not exceed
		// `MAX_VALUE_ALIGNMENT`
		let _ = AlignmentCheck::<T, MAX_VALUE_ALIGNMENT>::ASSERT_ALIGNMENT_DOES_NOT_EXCEED;

		// Align position in output buffer to alignment of `T`.
		// If `T`'s alignment requirement is less than or equal to `VALUE_ALIGNMENT`,
		// this can be skipped, as position is always left aligned to `VALUE_ALIGNMENT`
		// after each push.
		// This should be optimized away for types with alignment of `VALUE_ALIGNMENT`
		// or less, in which case this function becomes a no-op.
		// Hopefully this is the majority of types.
		if mem::align_of::<T>() > VALUE_ALIGNMENT {
			// Static assertion above ensures `align()`'s constraints are satisfied
			unsafe { self.align(mem::align_of::<T>()) }
		}
	}

	/// Align position in output buffer to `alignment`.
	///
	/// # Safety
	///
	/// * `alignment` must be `<= MAX_VALUE_ALIGNMENT`.
	/// * `alignment` must be a power of 2.
	#[inline]
	unsafe fn align(&mut self, alignment: usize) {
		debug_assert!(alignment <= MAX_VALUE_ALIGNMENT);
		debug_assert!(alignment.is_power_of_two());

		// Round up buffer position to multiple of `alignment`.
		// `align_up_to`'s constraints are satisfied by:
		// * `self.len()` is always less than `MAX_CAPACITY`, which is `< isize::MAX`.
		// * `alignment <= MAX_VALUE_ALIGNMENT` satisfies `alignment < isize::MAX`
		//   because `MAX_VALUE_ALIGNMENT < isize::MAX`.
		// * `alignment` being a power of 2 is part of this function's contract.
		let new_pos = align_up_to(self.len(), alignment);

		// `new_pos > capacity` can't happen because of 2 guarantees:
		// 1. `alignment <= MAX_VALUE_ALIGNMENT`
		// 2. `capacity` is a multiple of `MAX_VALUE_ALIGNMENT`
		self.set_len(new_pos);

		// TODO: Also align `current_block`
	}

	/// Align position in storage after pushing a `T` or slice `&[T]`.
	// TODO: This is a copy of `AlignedVec`'s method. De-dupe code.
	#[inline(always)] // Because this is generally a no-op
	fn align_after<T>(&mut self) {
		// Align buffer position to `VALUE_ALIGNMENT`, ready for the next value.
		// This should be optimized away for types with alignment of `VALUE_ALIGNMENT`
		// or greater. Ditto for types which have lower alignment, but happen to have
		// size divisible by `VALUE_ALIGNMENT`. Hopefully this is the majority of types.
		if mem::size_of::<T>() % VALUE_ALIGNMENT > 0 {
			self.align_after_any();
		}
	}

	/// Align position in storage after pushing values with
	/// `push_slice_unaligned`.
	///
	/// `align_after<T>` is often more efficient and can often be compiled down to
	/// a no-op, so is preferred.
	// TODO: This is a copy of `AlignedVec`'s method. De-dupe code.
	#[inline]
	fn align_after_any(&mut self) {
		// `VALUE_ALIGNMENT` trivially fulfills `align()`'s requirements
		unsafe { self.align(VALUE_ALIGNMENT) };
	}

	/// Reserve space in storage for `additional` bytes, growing capacity if
	/// required.
	#[inline]
	fn reserve(&mut self, additional: usize) {
		// Cannot wrap because capacity always exceeds len,
		// but avoids having to handle potential overflow here
		let remaining = self.capacity().wrapping_sub(self.len());
		if additional > remaining {
			self.grow_for_reserve(additional);
		}
	}

	/// Clear contents of storage.
	///
	/// Does not reduce the storage's capacity, just resets `len` back to 0.
	#[inline]
	fn clear(&mut self) {
		// TODO
		// NB: I imagine implementation *will* drop storage capacity
		// (contradicting the above doc comment).
	}

	/// Shrink the capacity of the storage as much as possible.
	/// `capacity` will be be a multiple of `MAX_VALUE_ALIGNMENT`.
	#[inline]
	fn shrink_to_fit(&mut self) {
		// TODO
	}
}

impl<
		const STORAGE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize,
	> AlignedBlocks<STORAGE_ALIGNMENT, MAX_VALUE_ALIGNMENT, VALUE_ALIGNMENT, MAX_CAPACITY>
{
	/// Grow storage to accomodate another `additional` bytes.
	///
	/// Separate function to guide inlining and branch prediction.
	///
	/// `additional` must not be 0 (which it can't be when called by `reserve`).
	#[cold]
	fn grow_for_reserve(&mut self, additional: usize) {
		// Calculate new total capacity
		// Increase total capacity to next power of 2 which is large enough so new block
		// can accomodate `additional` bytes
		let old_capacity = self.capacity;
		let new_capacity = old_capacity
			.checked_add(additional)
			.expect("Cannot grow capacity beyond `isize::MAX + 1`");
		assert!(
			new_capacity <= MAX_CAPACITY,
			"Cannot grow capacity beyond MAX_CAPACITY"
		);
		let new_capacity = new_capacity.next_power_of_two();

		let block_index = self.block_count;
		debug_assert!((block_index as usize) < self.blocks.len());
		debug_assert!((block_index as usize) < self.block_positions.len());

		// Create new block
		let new_block_capacity = new_capacity - old_capacity;
		let new_block = AlignedBytes::with_capacity(new_block_capacity);
		let old_block = mem::replace(&mut self.current_block, new_block);

		self.block_count += 1;
		self.capacity = new_capacity;

		// Record position in storage this block starts at (which is `old_capacity`)
		unsafe { *self.block_positions.get_unchecked_mut(block_index as usize) = old_capacity };

		// Store previous current block in `blocks`, unless it was an empty dummy
		if block_index > 0 {
			unsafe { *self.blocks.get_unchecked_mut(block_index as usize - 1) = old_block };

			// Set `block_indexes` for all magnitudes within this new block.
			// Skipped if `block_index == 0`, as `block_indexes` initialized as 0s anyway.

			// Example to check logic:
			// `old_capacity` = 4, `new_capacity` = 16, `block_index` = 1
			// -> `old_magnitude` = 62, `new_magnitude` = 60
			// -> Sets `block_indexes[60] = 1` and `block_indexes[61] = 1`.
			//    All other `block_indexes` values are 0.
			// Block index lookups with `get_block_index_and_offset_for_pos`:
			// * pos  0 -> magnitude 64 -> block index 0 (special case)
			// * pos  1 -> magnitude 63 -> block index 0
			// * pos  2 -> magnitude 62 -> block index 0
			// * pos  3 -> magnitude 62 -> block index 0
			// * pos  4 -> magnitude 61 -> block index 1
			// * pos  5 -> magnitude 61 -> block index 1
			// * pos  8 -> magnitude 60 -> block index 1
			// * pos 15 -> magnitude 60 -> block index 1
			// * pos 16 -> magnitude 59 -> block index 0 (wrong because `pos` out of bounds)

			// This isn't the first block, so both old and new capacity are non-zero
			let new_magnitude = unsafe { magnitude_for_non_zero(new_capacity) } + 1;
			let old_magnitude = unsafe { magnitude_for_non_zero(old_capacity) } + 1;
			for magnitude in new_magnitude..old_magnitude {
				// Impossible for `magnitude` to be out of bounds because `old_capacity` is
				// non-zero, so max `old_magnitude` is 64. Highest `magnitude` therefore is 63.
				unsafe { *self.block_indexes.get_unchecked_mut(magnitude) = block_index };
			}
		}
	}

	/// Translate position in storage to index of block holding that data,
	/// and offset of the data within that block.
	///
	/// `pos` must be within the bounds of the storage
	/// (i.e. `pos < storage.capacity()`).
	///
	/// `pos` where `pos == storage.capacity()` is specifically not supported.
	///
	/// Calling this method with a `pos` which violates above constraint will not
	/// be UB in itself (so this method is safe), but a later attempt to read from
	/// that invalid position may read the wrong data, or be an out of bounds
	/// access (UB).
	pub fn get_block_index_and_offset_for_pos(&self, pos: usize) -> (u8, usize) {
		debug_assert!(pos < self.capacity());

		// Handle 0 separately, to:
		// 1. Avoid `block_indexes` having to have length 65 (instead of 64).
		// 2. Allow using `NonZeroUsize::leading_zeros` which has better performance on
		//    some platforms.
		// `result_for_zero_pos` is in separate function to guide branch prediction.
		if pos == 0 {
			return result_for_zero_pos();
		}

		#[inline]
		#[cold]
		fn result_for_zero_pos() -> (u8, usize) {
			// Position 0 is always in 1st block, and 1st block always starts at 0
			(0, 0)
		}

		// `pos` cannot be 0 - that's handled above
		let magnitude = unsafe { magnitude_for_non_zero(pos) };
		unsafe {
			// Safe because `magnitude` cannot be greater than `MAX_BLOCK_COUNT - 1`.
			// Case where it would be (pos = 0) is handled above.
			debug_assert!(magnitude < self.block_indexes.len());
			let block_index = *self.block_indexes.get_unchecked(magnitude);

			// Logic elsewhere ensures all values in `block_indexes` are `< block_count`,
			// even for a `pos` which is out of bounds (`block_indexes` initialized as 0s).
			// In turn, `block_count` is `<= block_positions.len()`.
			// So this is safe for any value of `pos`, even invalid ones.
			debug_assert!(block_index < self.block_count);
			debug_assert!((block_index as usize) < self.block_positions.len());
			let block_pos = *self.block_positions.get_unchecked(block_index as usize);

			debug_assert!(pos >= block_pos);
			(block_index, pos - block_pos)
		}
	}
}

impl<
		const STORAGE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize,
	> AlignedStorage<STORAGE_ALIGNMENT, MAX_VALUE_ALIGNMENT, VALUE_ALIGNMENT, MAX_CAPACITY>
	for AlignedBlocks<STORAGE_ALIGNMENT, MAX_VALUE_ALIGNMENT, VALUE_ALIGNMENT, MAX_CAPACITY>
{
}

/// Create a boxed slice containing `count` default values.
// TODO: Check how Rust does this. Maybe it's better.
fn create_default_boxed_slice<T: Default>(count: usize) -> Box<[T]> {
	let mut vec = Vec::<T>::with_capacity(count);
	let ptr = vec.as_mut_ptr();
	unsafe {
		for i in 0..count {
			*ptr.add(i) = Default::default();
		}
		vec.set_len(count);
	}
	vec.into_boxed_slice()
}

/// Get `magnitude` of a non-zero position.
///
/// `magnitude` is max 64, but returns `usize` as that's how it's commonly used.
///
/// This uses `NonZeroUsize::leading_zeros` which is more performant on some
/// platforms.
///
/// # Safety
///
/// `pos` cannot be 0.
#[inline]
const unsafe fn magnitude_for_non_zero(pos: usize) -> usize {
	NonZeroUsize::new_unchecked(pos).leading_zeros() as usize
}
