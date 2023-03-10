use std::{cmp, marker::PhantomData, mem, ptr};

use super::{AlignedByteVec, ContiguousStorage, Storage};
use crate::util::{align_up_to, is_aligned_to};

/// Trait for storage used by Serializers which ensures values added to storage
/// maintain correct alignment in memory.
///
/// # Const parameters
///
/// * `MEMORY_ALIGNMENT`: alignment of the underlying memory used by `Storage`.
/// * `VALUE_ALIGNMENT`: minimum alignment all values will have in `Storage`.
/// 	Types with alignment higher than `VALUE_ALIGNMENT` will have padding
/// 	inserted before them if required. Types with alignment lower than
///   `VALUE_ALIGNMENT` will have padding inserted after to leave the `Storage`
/// 	aligned on `VALUE_ALIGNMENT` for the next `push()`.
/// * `MAX_VALUE_ALIGNMENT`: maximum alignment requirement of values which can
///   be stored in `Storage`. `capacity` must always be a multiple of this
/// 	(all methods uphold this constraint).
pub trait AlignedStorage<
	const MEMORY_ALIGNMENT: usize,
	const VALUE_ALIGNMENT: usize,
	const MAX_VALUE_ALIGNMENT: usize,
>: Storage
{
	/// Alignment of storage's memory buffer.
	const MEMORY_ALIGNMENT: usize = MEMORY_ALIGNMENT;

	/// Typical alignment of values being added to storage.
	const VALUE_ALIGNMENT: usize = VALUE_ALIGNMENT;

	/// Maximum alignment of values being added to storage.
	const MAX_VALUE_ALIGNMENT: usize = MAX_VALUE_ALIGNMENT;

	/// Maximum capacity of output buffer.
	/// Dictated by the requirements of
	/// [`alloc::Layout`](https://doc.rust-lang.org/alloc/alloc/struct.Layout.html).
	/// "`size`, when rounded up to the nearest multiple of `align`, must not
	/// overflow `isize` (i.e. the rounded value must be less than or equal to
	/// `isize::MAX`)".
	const MAX_CAPACITY: usize = isize::MAX as usize - (MEMORY_ALIGNMENT - 1);

	/// Assertions for validity of alignment const params.
	/// These assertions are not evaluated here.
	/// `Self::ASSERT_ALIGNMENTS_VALID` must be referenced in all code paths
	/// creating an `AlignedStorage`, to ensure compile-time error if
	/// assertions fail.
	const ASSERT_ALIGNMENTS_VALID: () = {
		assert!(MEMORY_ALIGNMENT > 0, "MEMORY_ALIGNMENT cannot be 0");
		assert!(
			MEMORY_ALIGNMENT < isize::MAX as usize,
			"MEMORY_ALIGNMENT must be less than isize::MAX"
		);
		assert!(
			MEMORY_ALIGNMENT.is_power_of_two(),
			"MEMORY_ALIGNMENT must be a power of 2"
		);
		assert!(MAX_VALUE_ALIGNMENT > 0, "MAX_VALUE_ALIGNMENT cannot be 0");
		assert!(
			MAX_VALUE_ALIGNMENT <= MEMORY_ALIGNMENT,
			"MAX_VALUE_ALIGNMENT must be less than or equal to ALIGNMENT",
		);
		assert!(
			MAX_VALUE_ALIGNMENT.is_power_of_two(),
			"MAX_VALUE_ALIGNMENT must be a power of 2"
		);
		assert!(VALUE_ALIGNMENT > 0, "VALUE_ALIGNMENT cannot be 0");
		assert!(
			VALUE_ALIGNMENT <= MAX_VALUE_ALIGNMENT,
			"VALUE_ALIGNMENT must be less than or equal to MAX_VALUE_ALIGNMENT",
		);
		assert!(
			VALUE_ALIGNMENT.is_power_of_two(),
			"VALUE_ALIGNMENT must be a power of 2"
		);
	};
}

/// Aligned contiguous memory buffer. Used by `BaseSerializer`.
///
/// A wrapper around rkyv's `AlignedByteVec` which ensures all values pushed to
/// the storage are correctly aligned.
///
/// See `AlignedStorage` trait for details of the const parameters.
pub struct AlignedVec<
	const MEMORY_ALIGNMENT: usize,
	const VALUE_ALIGNMENT: usize,
	const MAX_VALUE_ALIGNMENT: usize,
> {
	inner: AlignedByteVec<MEMORY_ALIGNMENT>,
}

impl<
		const MEMORY_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
	> Storage for AlignedVec<MEMORY_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT>
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

		// Round up capacity to multiple of `MAX_VALUE_ALIGNMENT`.
		// Assertion ensures overflow in `align_up_to()` is not possible.
		assert!(
			capacity <= Self::MAX_CAPACITY,
			"`capacity` cannot exceed isize::MAX - 15"
		);
		let capacity = align_up_to(capacity, MAX_VALUE_ALIGNMENT);

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
	/// * `capacity` must be a multiple of `MAX_VALUE_ALIGNMENT`.
	#[inline]
	unsafe fn with_capacity_unchecked(capacity: usize) -> Self {
		// Ensure (at compile time) that const params for alignment are valid
		let _ = Self::ASSERT_ALIGNMENTS_VALID;

		debug_assert!(is_aligned_to(capacity, MAX_VALUE_ALIGNMENT));

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
	/// * `new_len` must be a multiple of `VALUE_ALIGNMENT`.
	#[inline]
	unsafe fn set_len(&mut self, new_len: usize) {
		debug_assert!(new_len <= self.capacity());
		debug_assert!(is_aligned_to(new_len, VALUE_ALIGNMENT));

		self.inner.set_len(new_len);
	}

	/// Push a slice of values `&T` to storage, without alignment checks.
	///
	/// # Safety
	///
	/// This method does *not* ensure 2 invariants relating to alignment:
	///
	/// * `len` must be aligned for the type before push
	/// * `len` must be aligned to `VALUE_ALIGNMENT` after push
	///
	/// Caller must uphold these invariants. It is sufficient to:
	///
	/// * call `align_for::<T>()` before and
	/// * call `align_after::<T>()` after.
	#[inline]
	unsafe fn push_slice_unaligned<T>(&mut self, slice: &[T]) {
		debug_assert!(is_aligned_to(self.len(), mem::align_of::<T>()));

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
	/// Caller must ensure `AlignedVec` has sufficient capacity.
	///
	/// `size` must be total size in bytes of `&[T]`.
	/// i.e. `size = mem::size_of::<T>() * slice.len()`.
	///
	/// This method does *not* ensure 2 invariants of storage relating to
	/// alignment:
	///
	/// * that `len` is aligned for the type before push
	/// * that `len` is aligned to `VALUE_ALIGNMENT` after push
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

		let src = slice.as_ptr();
		let dst = self.as_mut_ptr().add(self.len()) as *mut T;
		// `src` must be correctly aligned as derived from a valid `&[T]`.
		// `dst` must be within this allocation as `len` can never exceed capacity.
		// Ensuring sufficient capacity is a requirement of this method.
		// `dst` being correctly aligned is a requirement of this method.
		ptr::copy_nonoverlapping(src, dst, slice.len());
		self.inner.set_len(self.len() + size);
	}

	/// Reserve capacity for at least `additional` more bytes to be inserted into
	/// the `AlignedVec`.
	///
	/// Growth of capacity occurs in powers of 2, and is always at minimum
	/// `MAX_VALUE_ALIGNMENT`.
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

	/// Align position in storage to alignment of `T`.
	#[inline]
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
	}

	/// Align position in storage after pushing a `T` or slice `&[T]`.
	#[inline]
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
	#[inline]
	fn align_after_any(&mut self) {
		// `VALUE_ALIGNMENT` trivially fulfills `align()`'s requirements
		unsafe { self.align(VALUE_ALIGNMENT) };
	}

	/// Clear contents of storage.
	///
	/// Does not reduce the storage's capacity, just resets `len` back to 0.
	#[inline]
	fn clear(&mut self) {
		self.inner.clear();
	}

	/// Shrink the capacity of the storage as much as possible.
	/// `capacity` will be be a multiple of `MAX_VALUE_ALIGNMENT`.
	#[inline]
	fn shrink_to_fit(&mut self) {
		// Ensure capacity remains a multiple of `MAX_VALUE_ALIGNMENT`
		let new_capacity = align_up_to(self.len(), MAX_VALUE_ALIGNMENT);

		if new_capacity != self.capacity() {
			// New capacity cannot exceed max as it's shrinking
			unsafe { self.inner.change_capacity(new_capacity) };
		}
	}
}

impl<
		const MEMORY_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
	> AlignedVec<MEMORY_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT>
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
	/// that `capacity` is always a multiple of `MAX_VALUE_ALIGNMENT`.
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
			// Ensuring at least `MAX_VALUE_ALIGNMENT` here makes sure capacity will always
			// remain a multiple of `MAX_VALUE_ALIGNMENT` hereafter, as growth after this
			// will be in powers of 2, and `shrink_to_fit` also enforces this invariant.
			// Calculations cannot overflow due to check above.
			cmp::max(new_cap.next_power_of_two(), MAX_VALUE_ALIGNMENT)
		};

		// Above calculation ensures `change_capacity`'s requirements are met
		unsafe { self.inner.change_capacity(new_cap) };
	}
}

impl<
		const MEMORY_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
	> ContiguousStorage for AlignedVec<MEMORY_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT>
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

impl<
		const MEMORY_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
	> AlignedStorage<MEMORY_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT>
	for AlignedVec<MEMORY_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT>
{
}

/// Type for static assertion that types being serialized do not have a higher
/// alignment requirement than the alignment of the output buffer
struct AlignmentCheck<T, const MAX_VALUE_ALIGNMENT: usize> {
	_marker: PhantomData<T>,
}

impl<T, const MAX_VALUE_ALIGNMENT: usize> AlignmentCheck<T, MAX_VALUE_ALIGNMENT> {
	const ASSERT_ALIGNMENT_DOES_NOT_EXCEED: () = assert!(mem::align_of::<T>() <= MAX_VALUE_ALIGNMENT);
}
