use std::cmp;

use super::{AlignedByteVec, ContiguousStorage, Storage};
use crate::util::align_up_to;

/// Trait for storage used by Serializers which has a specified alignment in
/// memory.
pub trait AlignedStorage<const ALIGNMENT: usize, const CAPACITY_ALIGNMENT: usize>: Storage {
	/// Alignment of storage's memory buffer.
	const ALIGNMENT: usize = ALIGNMENT;

	/// `capacity` must always be a multiple of this.
	const CAPACITY_ALIGNMENT: usize = CAPACITY_ALIGNMENT;

	/// Maximum capacity of output buffer.
	/// Dictated by the requirements of
	/// [`alloc::Layout`](https://doc.rust-lang.org/alloc/alloc/struct.Layout.html).
	/// "`size`, when rounded up to the nearest multiple of `align`, must not
	/// overflow `isize` (i.e. the rounded value must be less than or equal to
	/// `isize::MAX`)".
	const MAX_CAPACITY: usize = isize::MAX as usize - (ALIGNMENT - 1);

	/// Assertions for validity of alignment const params.
	/// These assertions are not evaluated here.
	/// `Self::ASSERT_ALIGNMENTS_VALID` must be referenced in all code paths
	/// creating an `AlignedStorage`, to ensure compile-time error if
	/// assertions fail.
	const ASSERT_ALIGNMENTS_VALID: () = {
		assert!(ALIGNMENT > 0, "ALIGNMENT cannot be 0");
		assert!(
			ALIGNMENT < isize::MAX as usize,
			"ALIGNMENT must be less than isize::MAX"
		);
		assert!(
			ALIGNMENT.is_power_of_two(),
			"ALIGNMENT must be a power of 2"
		);
		assert!(CAPACITY_ALIGNMENT > 0, "CAPACITY_ALIGNMENT cannot be 0");
		assert!(
			CAPACITY_ALIGNMENT <= ALIGNMENT,
			"CAPACITY_ALIGNMENT must be less than or equal to ALIGNMENT",
		);
		assert!(
			CAPACITY_ALIGNMENT.is_power_of_two(),
			"CAPACITY_ALIGNMENT must be a power of 2"
		);
	};
}

/// Aligned contiguous memory buffer. Used by `BaseSerializer`.
///
/// A wrapper around rkyv's `AlignedByteVec`.
///
/// Const params:
///
/// * `ALIGNMENT` is alignment of the underlying memory.
/// * `CAPACITY_ALIGNMENT` is amount that `capacity` must always be a multiple
///   of (this constraint is enforced in all methods).
pub struct AlignedVec<const ALIGNMENT: usize, const CAPACITY_ALIGNMENT: usize> {
	inner: AlignedByteVec<ALIGNMENT>,
}

impl<const ALIGNMENT: usize, const CAPACITY_ALIGNMENT: usize> Storage
	for AlignedVec<ALIGNMENT, CAPACITY_ALIGNMENT>
{
	/// Create new `AlignedVec`.
	#[inline]
	fn new() -> Self {
		// Ensure (at compile time) that const params for alignment are valid
		let _ = Self::ASSERT_ALIGNMENTS_VALID;

		Self {
			inner: AlignedByteVec::new(),
		}
	}

	/// Create new `AlignedVec` with pre-allocated capacity.
	#[inline]
	fn with_capacity(capacity: usize) -> Self {
		// Ensure (at compile time) that const params for alignment are valid
		let _ = Self::ASSERT_ALIGNMENTS_VALID;

		if capacity == 0 {
			return Self::new();
		}

		// Round up capacity to multiple of `CAPACITY_ALIGNMENT`.
		// Assertion ensures overflow in `align_up_to()` is not possible.
		assert!(
			capacity <= Self::MAX_CAPACITY,
			"`capacity` cannot exceed isize::MAX - 15"
		);
		let capacity = align_up_to(capacity, CAPACITY_ALIGNMENT);

		// TODO: `AlignedByteVec::with_capacity()` repeats the check if `capacity > 0`
		// and the assertion of max size. Would be good to skip that, but
		// `rkyv::AlignedByteVec` has no `with_capacity_unchecked` method.
		Self {
			inner: AlignedByteVec::with_capacity(capacity),
		}
	}

	/// Create new `AlignedVec` with pre-allocated capacity,
	/// without safety checks.
	///
	/// # Panics
	///
	/// Panics if `capacity` exceeds `Self::MAX_CAPACITY`.
	///
	/// # Safety
	///
	/// * `capacity` must be a multiple of `CAPACITY_ALIGNMENT`
	#[inline]
	unsafe fn with_capacity_unchecked(capacity: usize) -> Self {
		// Ensure (at compile time) that const params for alignment are valid
		let _ = Self::ASSERT_ALIGNMENTS_VALID;

		// `AlignedByteVec::with_capacity` panics if capacity exceeds max
		Self {
			inner: AlignedByteVec::with_capacity(capacity),
		}
	}

	/// Returns current capacity of storage in bytes.
	#[inline]
	fn capacity(&self) -> usize {
		self.inner.capacity()
	}

	/// Returns amount of storage currently used in bytes.
	#[inline]
	fn len(&self) -> usize {
		self.inner.len()
	}

	/// Set amount of storage space used (in bytes).
	///
	/// # Safety
	///
	/// * `new_len` must be less than or equal to `capacity()`.
	///
	/// If this storage instance is being used with `BaseSerializer`, additionally
	/// `new_len` must be a multiple of serializer's `VALUE_ALIGNMENT`.
	#[inline]
	unsafe fn set_len(&mut self, new_len: usize) {
		self.inner.set_len(new_len);
	}

	/// Reserve capacity for at least `additional` more bytes to be inserted into
	/// the `AlignedVec`.
	///
	/// Growth of capacity occurs in powers of 2, and is always at minimum
	/// `CAPACITY_ALIGNMENT`.
	///
	/// # Panics
	///
	/// Panics if the new capacity exceeds `isize::MAX - ALIGNMENT` bytes.
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
		self.inner.clear();
	}

	/// Shrink the capacity of the storage as much as possible.
	/// `capacity` will be be a multiple of `CAPACITY_ALIGNMENT`.
	#[inline]
	fn shrink_to_fit(&mut self) {
		// Ensure capacity remains a multiple of `CAPACITY_ALIGNMENT`
		let new_capacity = align_up_to(self.len(), CAPACITY_ALIGNMENT);

		if new_capacity != self.capacity() {
			// New capacity cannot exceed max as it's shrinking
			unsafe { self.inner.change_capacity(new_capacity) };
		}
	}
}

impl<const ALIGNMENT: usize, const CAPACITY_ALIGNMENT: usize>
	AlignedVec<ALIGNMENT, CAPACITY_ALIGNMENT>
{
	/// Extend capacity after `reserve` has found it's necessary.
	///
	/// Actually performing the extension is in this separate function marked
	/// `#[cold]` to hint to compiler that this branch is not often taken.
	/// This keeps the path for common case where capacity is already sufficient
	/// as fast as possible, and makes `reserve` more likely to be inlined.
	/// This is the same trick that Rust's `Vec::reserve` uses.
	///
	/// This is a copy of `rkyv::AlignedByteVec::do_reserve`, except it ensures
	/// that `capacity` is always a multiple of `CAPACITY_ALIGNMENT`.
	#[cold]
	fn grow_for_reserve(&mut self, additional: usize) {
		let new_cap = self
			.len()
			.checked_add(additional)
			.expect("Cannot grow AlignedVec further");

		let new_cap = if new_cap > (isize::MAX as usize + 1) >> 1 {
			// Rounding up to next power of 2 would result in `isize::MAX + 1` or higher,
			// which exceeds max capacity. So cap at max instead.
			assert!(
				new_cap <= Self::MAX_CAPACITY,
				"Cannot grow AlignedVec further"
			);
			Self::MAX_CAPACITY
		} else {
			// Ensuring at least `CAPACITY_ALIGNMENT` here makes sure capacity will always
			// remain a multiple of `CAPACITY_ALIGNMENT` hereafter, as growth after this
			// will be in powers of 2, and `shrink_to_fit` also enforces this invariant.
			// Calculations cannot overflow due to check above.
			cmp::max(new_cap.next_power_of_two(), CAPACITY_ALIGNMENT)
		};

		// Above calculation ensures `change_capacity`'s requirements are met
		unsafe { self.inner.change_capacity(new_cap) };
	}
}

impl<const ALIGNMENT: usize, const CAPACITY_ALIGNMENT: usize> ContiguousStorage
	for AlignedVec<ALIGNMENT, CAPACITY_ALIGNMENT>
{
	/// Returns a raw pointer to the storage's buffer, or a dangling raw pointer
	/// valid for zero sized reads if the storage didn't allocate.
	///
	/// The caller must ensure that the storage outlives the pointer this function
	/// returns, or else it will end up pointing to garbage. Modifying the storage
	/// may cause its buffer to be reallocated, which would also make any pointers
	/// to it invalid.
	#[inline]
	fn as_ptr(&self) -> *const u8 {
		self.inner.as_ptr()
	}

	/// Returns an unsafe mutable pointer to the storage's buffer, or a dangling
	/// raw pointer valid for zero sized reads if the storage didn't allocate.
	///
	/// The caller must ensure that the storage outlives the pointer this function
	/// returns, or else it will end up pointing to garbage. Modifying the storage
	/// may cause its buffer to be reallocated, which would also make any pointers
	/// to it invalid.
	#[inline]
	fn as_mut_ptr(&mut self) -> *mut u8 {
		self.inner.as_mut_ptr()
	}

	/// Extracts a slice containing the entire storage buffer.
	#[inline]
	fn as_slice(&self) -> &[u8] {
		self.inner.as_slice()
	}

	/// Extracts a mutable slice of the entire storage buffer.
	#[inline]
	fn as_mut_slice(&mut self) -> &mut [u8] {
		self.inner.as_mut_slice()
	}
}

impl<const ALIGNMENT: usize, const CAPACITY_ALIGNMENT: usize>
	AlignedStorage<ALIGNMENT, CAPACITY_ALIGNMENT> for AlignedVec<ALIGNMENT, CAPACITY_ALIGNMENT>
{
}
