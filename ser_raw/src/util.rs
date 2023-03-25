//! Utility functions related to alignment.

use std::mem;

/// Round up `pos` to alignment of `alignment`.
///
/// `alignment` must be a power of 2.
///
/// Caller must ensure `pos + alignment` cannot overflow `usize`.
/// This is satisfied if both `pos` and `alignment` are less than `isize::MAX`.
///
/// Breaking these conditions will yield an incorrect result which could
/// cause UB later on due to mis-aligned data.
#[inline]
pub const fn align_up_to(pos: usize, alignment: usize) -> usize {
	debug_assert!(alignment.is_power_of_two());
	(pos + alignment - 1) & !(alignment - 1)
}

/// Check if `pos` is a multiple of `alignment`.
///
/// `alignment` must be a power of 2.
#[inline]
pub const fn is_aligned_to(pos: usize, alignment: usize) -> bool {
	debug_assert!(alignment.is_power_of_two());
	pos & (alignment - 1) == 0
}

/// Get maximum maximum capacity for a [`Storage`] on this system.
///
/// i.e. the maximum allowable value for [`MAX_CAPACITY`] const parameter, given
/// the value chosen for [`MAX_VALUE_ALIGNMENT`] const parameter.
///
/// `alignment` must be a power of 2, less than `isize::MAX`.
///
/// Max capacity is dictated by the requirements of [`std::alloc::Layout`]:
///
/// > "`size`, when rounded up to the nearest multiple of `align`, must not
/// > overflow `isize` (i.e. the rounded value must be less than or equal to
/// > `isize::MAX`)".
///
/// [`Storage`]: crate::storage::Storage
/// [`MAX_CAPACITY`]: crate::storage::Storage::MAX_CAPACITY
/// [`MAX_VALUE_ALIGNMENT`]: crate::storage::Storage::MAX_VALUE_ALIGNMENT
/// [`std::alloc::Layout`]: https://doc.rust-lang.org/alloc/alloc/struct.Layout.html
pub const fn aligned_max_capacity(alignment: usize) -> usize {
	assert!(alignment != 0, "`alignment` cannot be 2");
	assert!(
		alignment.is_power_of_two(),
		"`alignment` must be a power of 2"
	);
	assert!(
		alignment < isize::MAX as usize,
		"`alignment` must be less than isize::MAX"
	);
	isize::MAX as usize - (alignment - 1)
}

/// Get maximum maximum capacity for an [`Storage`] on this system with a cap of
/// `u32::MAX + 1`.
///
/// Can be used to calculate a value for [`MAX_CAPACITY`] const parameter
/// whereby storage positions can always be expressed as a `u32`.
///
/// This will be:
/// * On 64-bit systems: `u32::MAX + 1` (i.e. 4 GiB)
/// * On 32-bit systems: `i32::MAX + 1 - alignment` (i.e. slighty below 2 GiB)
///
/// `alignment` must be a power of 2, less than `u32::MAX` and `isize::MAX`.
/// It should be the value used as [`MAX_VALUE_ALIGNMENT`] const parameter.
///
/// Cap at `i32::MAX + 1 - alignment` on 32-bit systems is dictated by the
/// requirements of [`std::alloc::Layout`]:
///
/// > "`size`, when rounded up to the nearest multiple of `align`, must not
/// > overflow `isize` (i.e. the rounded value must be less than or equal to
/// > `isize::MAX`)".
///
/// [`Storage`]: crate::storage::Storage
/// [`MAX_CAPACITY`]: crate::storage::Storage::MAX_CAPACITY
/// [`MAX_VALUE_ALIGNMENT`]: crate::storage::Storage::MAX_VALUE_ALIGNMENT
/// [`std::alloc::Layout`]: https://doc.rust-lang.org/alloc/alloc/struct.Layout.html
pub const fn aligned_max_u32_capacity(alignment: usize) -> usize {
	assert!(alignment != 0, "`alignment` cannot be 0");
	assert!(
		alignment.is_power_of_two(),
		"`alignment` must be a power of 2"
	);
	assert!(
		alignment < u32::MAX as usize && alignment < isize::MAX as usize,
		"`alignment` must be less than u32::MAX and isize::MAX"
	);

	if mem::size_of::<usize>() >= 8 {
		// This would overflow on a 32-bit system, but check above avoids this path
		// TODO: This may still fail to compile on 32-bit systems if compiler doesn't
		// understand this branch cannot be taken. Check this.
		u32::MAX as usize + 1
	} else {
		isize::MAX as usize - (alignment - 1)
	}
}
