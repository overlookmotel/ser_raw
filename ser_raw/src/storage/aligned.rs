use std::{
	cmp,
	marker::PhantomData,
	mem::{self, MaybeUninit},
	ptr, slice,
};

use super::{AlignedByteVec, ContiguousStorage, Storage};
use crate::util::{align_up_to, aligned_max_capacity, is_aligned_to};

const PTR_SIZE: usize = mem::size_of::<usize>();
const DEFAULT_STORAGE_ALIGNMENT: usize = 16;
const DEFAULT_VALUE_ALIGNMENT: usize = PTR_SIZE;
const DEFAULT_MAX_CAPACITY: usize = aligned_max_capacity(DEFAULT_STORAGE_ALIGNMENT);

/// Trait for storage used by [`Serializer`]s which ensures values added to
/// storage maintain correct alignment in memory for their types.
///
/// # Const parameters
///
/// By configuring alignment requirements statically, the compiler is able to
/// remove alignment calculations for many cases. This improves performance.
///
/// ## `STORAGE_ALIGNMENT`
///
/// Alignment of the underlying memory used by `Storage`.
///
/// * Must be a power of 2.
///
/// Default: 16
///
/// ## `MAX_VALUE_ALIGNMENT`
///
/// Maximum alignment requirement of values which can be stored in `Storage`.
/// Types with alignment greater than `MAX_VALUE_ALIGNMENT` cannot be stored in
/// this `Storage`.
///
/// * Must be a power of 2.
/// * Must be less than or equal to `STORAGE_ALIGNMENT`.
///
/// `capacity` will always be a multiple of this.
///
/// Default: `STORAGE_ALIGNMENT`
///
/// ## `VALUE_ALIGNMENT`
///
/// Minimum alignment all values will have in `Storage`.
///
/// Types with alignment higher than `VALUE_ALIGNMENT` will have padding
/// inserted before them if required. Types with alignment lower than
/// `VALUE_ALIGNMENT` will have padding inserted after them to leave the
/// `Storage` aligned on `VALUE_ALIGNMENT`, ready for the next `push()`.
///
/// This doesn't affect the "legality" of the output, but if most allocated
/// types being serialized have the same alignment, setting `VALUE_ALIGNMENT` to
/// that alignment may significantly improve performance, as alignment
/// calculations can be skipped when serializing those types.
///
/// NB: The word "allocated" in "allocated types" is key here. `ser_raw` deals
/// in allocations, not individual types. So this means that only types which
/// are pointed to by a `Box<T>` or `Vec<T>` count as "allocated types"
/// for the purposes of calculating an optimal value for `VALUE_ALIGNMENT`.
///
/// e.g. If all (or almost all) types contain pointers (`Box`, `Vec` etc),
/// setting `VALUE_ALIGNMENT = std::mem::size_of::<usize>()`
/// will be the best value for fast serialization.
///
/// The higher `VALUE_ALIGNMENT` is, the more padding bytes will end up in
/// output, potentially increasing output size, depending on the types being
/// serialized.
///
/// * Must be a power of 2.
/// * Must be less than or equal to `MAX_VALUE_ALIGNMENT`.
///
/// Default:
///
/// * 64-bit systems: 8
/// * 32-bit systems: 4
///
/// ## `MAX_CAPACITY`
///
/// Maximum capacity of storage.
///
/// * Cannot be 0.
/// * Cannot be greater than `isize::MAX + 1 - STORAGE_ALIGNMENT`.
/// * Must be a multiple of `MAX_VALUE_ALIGNMENT`.
///
/// Default:
///
/// * 64-bit systems: `u64::MAX - 15`
/// * 32-bit systems: `u32::MAX - 15`
///
/// [`Serializer`]: crate::Serializer
pub trait AlignedStorage<
	const STORAGE_ALIGNMENT: usize,
	const MAX_VALUE_ALIGNMENT: usize,
	const VALUE_ALIGNMENT: usize,
	const MAX_CAPACITY: usize,
>: Storage
{
	/// Alignment of storage's memory buffer.
	const STORAGE_ALIGNMENT: usize = STORAGE_ALIGNMENT;

	/// Maximum alignment of values being added to storage.
	const MAX_VALUE_ALIGNMENT: usize = MAX_VALUE_ALIGNMENT;

	/// Typical alignment of values being added to storage.
	const VALUE_ALIGNMENT: usize = VALUE_ALIGNMENT;

	/// Maximum capacity of storage.
	const MAX_CAPACITY: usize = MAX_CAPACITY;

	/// Assertions for validity of alignment const params.
	/// These assertions are not evaluated here.
	/// `Self::ASSERT_ALIGNMENTS_VALID` must be referenced in all code paths
	/// creating an `AlignedStorage`, to ensure compile-time error if
	/// assertions fail.
	const ASSERT_ALIGNMENTS_VALID: () = {
		assert!(STORAGE_ALIGNMENT > 0, "STORAGE_ALIGNMENT cannot be 0");
		assert!(
			STORAGE_ALIGNMENT < isize::MAX as usize,
			"STORAGE_ALIGNMENT must be less than isize::MAX"
		);
		assert!(
			STORAGE_ALIGNMENT.is_power_of_two(),
			"STORAGE_ALIGNMENT must be a power of 2"
		);

		assert!(MAX_VALUE_ALIGNMENT > 0, "MAX_VALUE_ALIGNMENT cannot be 0");
		assert!(
			MAX_VALUE_ALIGNMENT <= STORAGE_ALIGNMENT,
			"MAX_VALUE_ALIGNMENT must be less than or equal to STORAGE_ALIGNMENT",
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

		assert!(MAX_CAPACITY > 0, "MAX_CAPACITY cannot be 0");
		assert!(
			MAX_CAPACITY <= aligned_max_capacity(STORAGE_ALIGNMENT),
			"MAX_CAPACITY cannot exceed isize::MAX + 1 - STORAGE_ALIGNMENT"
		);
		assert!(
			MAX_CAPACITY % MAX_VALUE_ALIGNMENT == 0,
			"MAX_CAPACITY must be a multiple of MAX_VALUE_ALIGNMENT"
		);
	};
}

/// Aligned contiguous memory buffer. Used as backing storage by all of the
/// Serializers provided by this crate, except for [`UnalignedSerializer`].
///
/// A wrapper around rkyv's `AlignedByteVec` which ensures all values pushed to
/// the storage are correctly aligned.
///
/// See [`AlignedStorage`] trait for details of the const parameters.
///
/// [`UnalignedSerializer`]: crate::UnalignedSerializer
pub struct AlignedVec<
	const STORAGE_ALIGNMENT: usize = DEFAULT_STORAGE_ALIGNMENT,
	const MAX_VALUE_ALIGNMENT: usize = STORAGE_ALIGNMENT,
	const VALUE_ALIGNMENT: usize = DEFAULT_VALUE_ALIGNMENT,
	const MAX_CAPACITY: usize = DEFAULT_MAX_CAPACITY,
> {
	inner: AlignedByteVec<STORAGE_ALIGNMENT>,
}

impl<
		const STORAGE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize,
	> Storage for AlignedVec<STORAGE_ALIGNMENT, MAX_VALUE_ALIGNMENT, VALUE_ALIGNMENT, MAX_CAPACITY>
{
	/// Create new [`AlignedVec`].
	#[inline]
	fn new() -> Self {
		// Ensure (at compile time) that const params are valid
		let _ = Self::ASSERT_ALIGNMENTS_VALID;

		Self {
			inner: AlignedByteVec::new(),
		}
	}

	/// Create new [`AlignedVec`] with pre-allocated capacity.
	///
	/// Capacity will be rounded up to a multiple of `MAX_VALUE_ALIGNMENT`.
	///
	/// # Panics
	///
	/// Panics if `capacity` exceeds `MAX_CAPACITY`.
	fn with_capacity(capacity: usize) -> Self {
		// Ensure (at compile time) that const params are valid
		let _ = Self::ASSERT_ALIGNMENTS_VALID;

		if capacity == 0 {
			return Self::new();
		}

		// Round up capacity to multiple of `MAX_VALUE_ALIGNMENT`.
		// Assertion ensures overflow in `align_up_to()` is not possible.
		assert!(
			capacity <= MAX_CAPACITY,
			"capacity cannot exceed MAX_CAPACITY"
		);
		let capacity = align_up_to(capacity, MAX_VALUE_ALIGNMENT);

		// TODO: `AlignedByteVec::with_capacity()` repeats the check if `capacity > 0`
		// and the assertion of max size. Would be good to skip that, but
		// `rkyv::AlignedByteVec` has no `with_capacity_unchecked` method.
		Self {
			inner: AlignedByteVec::with_capacity(capacity),
		}
	}

	/// Create new [`AlignedVec`] with pre-allocated capacity,
	/// without safety checks.
	///
	/// # Safety
	///
	/// * `capacity` must be less than or equal to `MAX_CAPACITY`.
	/// * `capacity` must be a multiple of `MAX_VALUE_ALIGNMENT`.
	unsafe fn with_capacity_unchecked(capacity: usize) -> Self {
		// Ensure (at compile time) that const params are valid
		let _ = Self::ASSERT_ALIGNMENTS_VALID;

		debug_assert!(
			capacity <= MAX_CAPACITY,
			"capacity cannot exceed MAX_CAPACITY"
		);
		debug_assert!(is_aligned_to(capacity, MAX_VALUE_ALIGNMENT));

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
	/// Caller must ensure [`AlignedVec`] has sufficient capacity.
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

		self.write_slice(slice, self.len());
		self.inner.set_len(self.len() + size);
	}

	/// Reserve capacity for at least `additional` more bytes to be inserted into
	/// the [`AlignedVec`].
	///
	/// Growth of capacity occurs in powers of 2 up to `MAX_CAPACITY`, and is
	/// always at minimum `MAX_VALUE_ALIGNMENT`.
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
	}

	/// Align position in storage after pushing a `T` or slice `&[T]`.
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
		const STORAGE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize,
	> AlignedVec<STORAGE_ALIGNMENT, MAX_VALUE_ALIGNMENT, VALUE_ALIGNMENT, MAX_CAPACITY>
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
	/// that `capacity` is always a multiple of `MAX_VALUE_ALIGNMENT`
	/// and less than user-defined `MAX_CAPACITY`.
	#[cold]
	fn grow_for_reserve(&mut self, additional: usize) {
		// Where `reserve` was called by `push_slice_unaligned`, we could actually avoid
		// the checked add. A valid slice cannot be larger than `isize::MAX`, and ditto
		// `capacity`, so this can't overflow.
		// TODO: Maybe create a specialized version of this function for that usage?
		let new_cap = self
			.len()
			.checked_add(additional)
			.expect("Cannot grow AlignedVec further");

		let new_cap = if new_cap > MAX_CAPACITY.next_power_of_two() >> 1 {
			// Rounding up to next power of 2 would result in more than `MAX_CAPACITY`,
			// so cap at max instead.
			assert!(new_cap <= MAX_CAPACITY, "Cannot grow AlignedVec further");
			MAX_CAPACITY
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
		const STORAGE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize,
	> ContiguousStorage
	for AlignedVec<STORAGE_ALIGNMENT, MAX_VALUE_ALIGNMENT, VALUE_ALIGNMENT, MAX_CAPACITY>
{
	/// Write a slice of values at a specific position in storage's buffer.
	///
	/// # Safety
	///
	/// * Storage `capacity` must be greater or equal to
	/// 	`pos + std::mem::size_of::<T>() * slice.len()`.
	/// 	i.e. write is within storage's allocation.
	/// * `pos` must be aligned for `T`.
	#[inline]
	unsafe fn write_slice<T>(&mut self, slice: &[T], pos: usize) {
		debug_assert!(pos <= self.capacity());
		debug_assert!(self.capacity() - pos >= mem::size_of::<T>() * slice.len());
		debug_assert!(is_aligned_to(pos, mem::align_of::<T>()));

		// Do nothing if ZST. This function will be compiled down to a no-op for ZSTs.
		if mem::size_of::<T>() == 0 {
			return;
		}

		let src = slice.as_ptr();
		let dst = self.as_mut_ptr().add(pos) as *mut T;
		// `src` must be correctly aligned as derived from a valid `&[T]`.
		// Ensuring sufficient capacity is a requirement of this method.
		// `dst` being correctly aligned is a requirement of this method.
		ptr::copy_nonoverlapping(src, dst, slice.len());
	}

	/// Returns a raw pointer to the storage's buffer, or a dangling raw pointer
	/// valid for zero sized reads if the storage didn't allocate.
	///
	/// The caller must ensure that the storage outlives the pointer this function
	/// returns, or else it will end up pointing to garbage. Modifying the storage
	/// may cause its buffer to be reallocated, which would also make any pointers
	/// to it invalid.
	///
	/// Buffer will very likely contain uninitialized padding bytes. It's safe to
	/// read from the sections of the buffer which are initialized, but reading
	/// the uninitialized bytes is UB. Understanding which are which requires an
	/// exact knowledge of the memory layouts of types which have been written to
	/// this storage.
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
	///
	/// Buffer will very likely contain uninitialized padding bytes. It's safe to
	/// read from the sections of the buffer which are initialized, but reading
	/// the uninitialized bytes is UB. Understanding which are which requires an
	/// exact knowledge of the memory layouts of types which have been written to
	/// this storage.
	#[inline]
	fn as_mut_ptr(&mut self) -> *mut u8 {
		self.inner.as_mut_ptr()
	}

	/// Extracts a slice containing the entire storage buffer.
	///
	/// Buffer will very likely contain uninitialized padding bytes. It's safe to
	/// read from the sections of the buffer which are initialized, but reading
	/// the uninitialized bytes is UB. Understanding which are which requires an
	/// exact knowledge of the memory layouts of types which have been written to
	/// this storage.
	#[inline]
	fn as_slice(&self) -> &[MaybeUninit<u8>] {
		unsafe { slice::from_raw_parts(self.as_ptr() as *const MaybeUninit<u8>, self.len()) }
	}

	/// Extracts a mutable slice of the entire storage buffer.
	///
	/// Buffer will very likely contain uninitialized padding bytes. It's safe to
	/// read from the sections of the buffer which are initialized, but reading
	/// the uninitialized bytes is UB. Understanding which are which requires an
	/// exact knowledge of the memory layouts of types which have been written to
	/// this storage.
	#[inline]
	fn as_mut_slice(&mut self) -> &mut [MaybeUninit<u8>] {
		unsafe { slice::from_raw_parts_mut(self.as_mut_ptr() as *mut MaybeUninit<u8>, self.len()) }
	}
}

impl<
		const STORAGE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize,
	> AlignedStorage<STORAGE_ALIGNMENT, MAX_VALUE_ALIGNMENT, VALUE_ALIGNMENT, MAX_CAPACITY>
	for AlignedVec<STORAGE_ALIGNMENT, MAX_VALUE_ALIGNMENT, VALUE_ALIGNMENT, MAX_CAPACITY>
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
