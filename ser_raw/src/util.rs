/// Round up `pos` to alignment of `alignment`.
///
/// `alignment` must be a power of 2.
///
/// Caller must ensure `pos + alignment` cannot overflow `usize`.
/// This is satisfied if both `pos` and `alignment` are less than `isize::MAX`.
///
/// Breaking these conditions will yield an incorrect result which could
/// cause UB later on due to mis-aligned data.
pub const fn align_up_to(pos: usize, alignment: usize) -> usize {
	debug_assert!(alignment.is_power_of_two());
	(pos + alignment - 1) & !(alignment - 1)
}

/// Check if `pos` is a multiple of `alignment`.
///
/// `alignment` must be a power of 2.
pub const fn is_aligned_to(pos: usize, alignment: usize) -> bool {
	debug_assert!(alignment.is_power_of_two());
	pos & (alignment - 1) == 0
}
