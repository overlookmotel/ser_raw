use std::{mem, num::NonZeroUsize};

use super::AlignedBytes;

const MAX_BLOCK_COUNT: usize = mem::size_of::<usize>() * 8;
const MAX_CAPACITY: usize = (isize::MAX as usize) + 1;

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
pub struct AlignedBlocks {
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

impl AlignedBlocks {
	/// Create new [`AlignedBlocks`] with no pre-allocated capacity.
	///
	/// A first block of memory will be allocated when is first pushed to.
	///
	/// To avoid creating lots of small blocks, it's recommended to use
	/// [`with_capacity`](AlignedBlocks::with_capacity) instead.
	pub fn new() -> Self {
		Self {
			capacity: 0,
			len: 0,
			block_count: 0,
			current_block: AlignedBytes::new(),
			blocks: create_default_boxed_slice::<AlignedBytes>(MAX_BLOCK_COUNT),
			block_positions: create_default_boxed_slice::<usize>(MAX_BLOCK_COUNT),
			block_indexes: Box::new([0; MAX_BLOCK_COUNT]),
		}
	}

	/// Create new [`AlignedBlocks`] with pre-allocated capacity.
	///
	/// The larger the initial capacity allocated, the less data will be split
	/// across multiple blocks, which is preferable for performance.
	pub fn with_capacity(capacity: usize) -> Self {
		if capacity == 0 {
			return Self::new();
		}

		// Round up capacity to next power of 2.
		// Cannot allocate `isize::MAX + 1` in a single allocation due to requirement of
		// `std::alloc::Layout`, so limit first allocation to `(isize::MAX + 1) / 2`.
		assert!(
			capacity <= MAX_CAPACITY / 2,
			"Requested capacity exceeds maximum for first allocation"
		);
		let capacity = capacity.next_power_of_two();

		// `capacity` cannot be zero (checked above)
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
	pub fn capacity(&self) -> usize {
		self.capacity
	}

	/// Returns amount of storage currently used in this [`AlignedBlocks`] in
	/// bytes.
	pub fn len(&self) -> usize {
		self.len
	}

	/// Reserve space in storage for `additional` bytes, growing capacity if
	/// required.
	pub fn reserve(&mut self, additional: usize) {
		if self.current_block.capacity() - self.current_block.len() < additional {
			self.grow_for_reserve(additional);
		}
	}

	/// Grow storage to accomodate another `additional bytes.
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
		assert!(new_capacity <= MAX_CAPACITY);
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

	// TODO: All the other `Storage` methods
}

/// Create a boxed slice containing `count` default values.
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
