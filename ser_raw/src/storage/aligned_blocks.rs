use std::{cmp, mem};

use super::AlignedBytes;

const MAX_BLOCK_COUNT: usize = mem::size_of::<usize>() * 8;
const MIN_CAPACITY: usize = 2;

// TODO: The logic around magnitudes is broken. Needs more thought.

/// Aligned storage which allocates memory in a series of blocks.
///
/// Each block is allocated as required, and then never grows, so all data in
/// the storage maintains a stable memory address.
///
/// Total capacity is always a power of 2, and grows by doubling total capacity
/// at minimum. Capacity can grow in larger jumps if a value larger than the
/// current capacity needs to be stored.
///
/// Only exception to the power of 2 rule is for capacities above
/// `isize::MAX + 1`, which are rounded up to `usize::MAX` instead.
///
/// Any capacity up to `usize::MAX` can be accomodated by a maximum of 64 blocks
/// (32 on 32-bit systems).
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
/// 64 blocks can fulfill any storage capacity up to `usize::MAX` if minimum
/// size for first block is 2 (or 32 blocks on 32-bit systems). The maximum
/// number of blocks required is lower if the [`AlignedBlocks`] is initialized
/// with a larger starting capacity.
///
/// A position's `magnitude` is the number of leading zeros it has in its binary
/// representation. Obtaining `magnitude` is a single processor instruction.
///
/// Position is a `usize`, so `magnitude` has an upper bound of
/// `mem::size_of::<usize>() * 8` (i.e. 64 on a 64-bit system).
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
/// </details>
///
/// [`with_capacity`]: AlignedBlocks::with_capacity
//
// TODO: Reduce size of this struct. It's currently 152 bytes, and it's
// non-optimal for structs to be larger than 128. Need to lose 24 bytes.
// Could:
// 1. Combine `blocks` and `block_positions` into a single `Vec`.
// 2. Store `block_indexes` as a `Box<[u8; 64]>`.
//
// Or does this really matter? I think it's only bad if the struct is moved,
// which in most cases it won't be.
pub struct AlignedBlocks {
	/// Total current capacity of storage
	capacity: usize,
	/// Total used storage
	len: usize,
	/// Current block which new pushes will add to
	current_block: AlignedBytes,
	/// Past blocks which are now full
	blocks: Vec<AlignedBytes>,
	/// Positions blocks start at (indexed by block index)
	block_positions: Vec<usize>,
	/// Mapping from position magnitude to block index
	block_indexes: [u8; MAX_BLOCK_COUNT],
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
			current_block: AlignedBytes::new(),
			blocks: Vec::with_capacity(MAX_BLOCK_COUNT),
			block_positions: Vec::with_capacity(MAX_BLOCK_COUNT),
			block_indexes: [0; MAX_BLOCK_COUNT],
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

		// Round up capacity to next power of 2
		let capacity = Self::round_up_capacity(capacity);

		// TODO: Is `+2` correct?
		let max_num_blocks = capacity.leading_zeros() as usize + 2;

		let mut block_positions = Vec::with_capacity(max_num_blocks);
		block_positions.push(0);

		Self {
			capacity,
			len: 0,
			current_block: AlignedBytes::with_capacity(capacity),
			blocks: Vec::with_capacity(max_num_blocks),
			block_positions,
			block_indexes: [0; MAX_BLOCK_COUNT],
		}
	}

	/// Round up capacity to next power of 2.
	/// If that would overflow, cap capacity at `usize::MAX` instead.
	fn round_up_capacity(capacity: usize) -> usize {
		if capacity > isize::MAX as usize + 1 {
			// Increasing to next power of 2 would overflow, so cap at `usize::MAX` instead
			usize::MAX
		} else {
			// `MIN_CAPACITY` ensures we can never require more than `MAX_BLOCK_COUNT`
			// blocks for any capacity up to `usize::MAX`
			cmp::max(capacity.next_power_of_two(), MIN_CAPACITY)
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
			.expect("Cannot grow capacity beyond `usize::MAX`");
		let new_capacity = Self::round_up_capacity(new_capacity);
		self.capacity = new_capacity;

		// Create new block
		let new_block_capacity = new_capacity - old_capacity;
		let new_block = AlignedBytes::with_capacity(new_block_capacity);
		let old_block = mem::replace(&mut self.current_block, new_block);
		self.block_positions.push(old_capacity);

		// Push previous current block to `blocks` unless it was a dummy
		if old_capacity > 0 {
			self.blocks.push(old_block);
		}

		// Set `block_indexes` for all magnitudes within this new block
		let block_index = self.blocks.len() as u8;
		// TODO: `new_magnitude` will be wrong if capacity = `usize::MAX`
		let new_magnitude = new_capacity.leading_zeros() as usize;
		let old_magnitude = old_capacity.leading_zeros() as usize;
		for magnitude in new_magnitude..old_magnitude {
			unsafe {
				// Impossible for `magnitude` to be out of bounds
				*self.block_indexes.get_unchecked_mut(magnitude) = block_index;
			}
		}
	}

	/// Translate position in storage to index of block holding that data,
	/// and offset of the data within that block
	pub fn get_block_index_and_offset_for_pos(&self, pos: usize) -> (u8, usize) {
		// TODO: This isn't right.
		// If `pos = 0`, `magnitude = 64`, which is out of bounds.
		let magnitude = pos.leading_zeros() as usize;
		unsafe {
			debug_assert!(magnitude < self.block_indexes.len());
			let block_index = *self.block_indexes.get_unchecked(magnitude);
			debug_assert!((block_index as usize) < self.block_positions.len());
			debug_assert!(pos >= self.block_positions[block_index as usize]);
			let pos_in_block = pos - self.block_positions.get_unchecked(block_index as usize);
			(block_index, pos_in_block)
		}
	}

	// TODO: All the other `Storage` methods
}
