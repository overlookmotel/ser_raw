use std::{
	alloc::{self, Layout},
	mem,
	ptr::{self, NonNull},
};

use super::{ContiguousStorage, PinnedStorage, RandomAccessStorage, Storage};
use crate::util::{aligned_max_capacity, is_aligned_to};

const PTR_SIZE: usize = mem::size_of::<usize>();
const DEFAULT_STORAGE_ALIGNMENT: usize = 16;
const DEFAULT_VALUE_ALIGNMENT: usize = PTR_SIZE;
const DEFAULT_MAX_CAPACITY: usize = aligned_max_capacity(DEFAULT_STORAGE_ALIGNMENT);

/// Aligned contiguous memory buffer which has fixed size and cannot grow.
///
/// Consequently, it maintains an unchangable memory location.
///
/// Ensures all values pushed to storage are correctly aligned.
///
/// Supports random access reads and writes via [`RandomAccessStorage`] trait.
///
/// See [`Storage`] trait for details of the const parameters.
///
/// # Example
///
/// ```
/// use ser_raw::storage::{AlignedBytes, ContiguousStorage, Storage};
///
/// let mut storage: AlignedBytes = AlignedBytes::with_capacity(8);
///
/// // Storage is aligned to [`STORAGE_ALIGNMENT`] (default 16)
/// assert!(storage.as_ptr() as usize % 16 == 0);
///
/// // Capacity is rounded up to multiple of [`MAX_VALUE_ALIGNMENT`] (default 16)
/// assert_eq!(storage.pos(), 0);
/// assert_eq!(storage.capacity(), 16);
///
/// let value: u32 = 100;
/// storage.push(&value);
///
/// // `pos` is rounded up to multiple of [`VALUE_ALIGNMENT`] (default 8)
/// assert_eq!(storage.pos(), 8);
/// assert_eq!(storage.capacity(), 16);
///
/// let slice: &[u64] = &vec![200];
/// storage.push_slice(slice);
///
/// // Storage is now full
/// assert_eq!(storage.pos(), 16);
/// assert_eq!(storage.capacity(), 16);
///
/// // This would panic
/// // storage.push(&0u8);
/// ```
///
/// [`STORAGE_ALIGNMENT`]: AlignedBytes::STORAGE_ALIGNMENT
/// [`MAX_VALUE_ALIGNMENT`]: AlignedBytes::MAX_VALUE_ALIGNMENT
/// [`VALUE_ALIGNMENT`]: AlignedBytes::VALUE_ALIGNMENT
pub struct AlignedBytes<
	const STORAGE_ALIGNMENT: usize = DEFAULT_STORAGE_ALIGNMENT,
	const MAX_VALUE_ALIGNMENT: usize = STORAGE_ALIGNMENT,
	const VALUE_ALIGNMENT: usize = DEFAULT_VALUE_ALIGNMENT,
	const MAX_CAPACITY: usize = DEFAULT_MAX_CAPACITY,
> {
	ptr: NonNull<u8>,
	capacity: usize,
	pos: usize,
}

impl<
		const STORAGE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize,
	> Storage for AlignedBytes<STORAGE_ALIGNMENT, MAX_VALUE_ALIGNMENT, VALUE_ALIGNMENT, MAX_CAPACITY>
{
	/// Alignment of storage's memory buffer.
	///
	/// See [`Storage`] trait for explanation.
	const STORAGE_ALIGNMENT: usize = STORAGE_ALIGNMENT;

	/// Maximum alignment of values being added to storage.
	///
	/// See [`Storage`] trait for explanation.
	const MAX_VALUE_ALIGNMENT: usize = MAX_VALUE_ALIGNMENT;

	/// Typical alignment of values being added to storage.
	///
	/// See [`Storage`] trait for explanation.
	const VALUE_ALIGNMENT: usize = VALUE_ALIGNMENT;

	/// Maximum capacity of storage.
	///
	/// See [`Storage`] trait for explanation.
	const MAX_CAPACITY: usize = MAX_CAPACITY;

	/// Create new [`AlignedBytes`] with zero capacity.
	///
	/// Does not allocate any memory.
	///
	/// [`AlignedBytes`] cannot grow, so any call to [`push`] or [`push_slice`]
	/// will exceed capacity and panic. Therefore this is only useful for creating
	/// a dummy placeholder.
	///
	/// [`push`]: AlignedBytes::push
	/// [`push_slice`]: AlignedBytes::push_slice
	#[inline]
	fn new() -> Self {
		// Ensure (at compile time) that const params are valid
		let _ = Self::ASSERT_ALIGNMENTS_VALID;

		Self {
			ptr: NonNull::dangling(),
			capacity: 0,
			pos: 0,
		}
	}

	/// Create new [`AlignedBytes`] with pre-allocated capacity,
	/// without safety checks.
	///
	/// Capacity cannot grow beyond this initial size.
	///
	/// # Safety
	///
	/// * `capacity` must not be 0.
	/// * `capacity` must be less than or equal to [`MAX_CAPACITY`].
	/// * `capacity` must be a multiple of [`MAX_VALUE_ALIGNMENT`].
	///
	/// [`MAX_CAPACITY`]: AlignedBytes::MAX_CAPACITY
	/// [`MAX_VALUE_ALIGNMENT`]: AlignedBytes::MAX_VALUE_ALIGNMENT
	unsafe fn with_capacity_unchecked(capacity: usize) -> Self {
		// Ensure (at compile time) that const params are valid
		let _ = Self::ASSERT_ALIGNMENTS_VALID;

		debug_assert!(capacity > 0, "capacity cannot be 0");
		debug_assert!(
			capacity <= MAX_CAPACITY,
			"capacity cannot exceed MAX_CAPACITY"
		);
		debug_assert!(is_aligned_to(capacity, MAX_VALUE_ALIGNMENT));

		let layout = Layout::from_size_align_unchecked(capacity, STORAGE_ALIGNMENT);
		let ptr = alloc::alloc(layout);
		if ptr.is_null() {
			alloc::handle_alloc_error(layout);
		}

		Self {
			ptr: NonNull::new_unchecked(ptr),
			capacity,
			pos: 0,
		}
	}

	/// Returns current capacity of storage in bytes.
	#[inline]
	fn capacity(&self) -> usize {
		self.capacity
	}

	/// Returns current position in storage.
	#[inline]
	fn pos(&self) -> usize {
		self.pos
	}

	/// Set current position in storage.
	///
	/// # Safety
	///
	/// * `new_pos` must be less than or equal to [`capacity()`].
	/// * `new_pos` must be a multiple of [`VALUE_ALIGNMENT`].
	///
	/// [`capacity()`]: AlignedBytes::capacity
	/// [`VALUE_ALIGNMENT`]: AlignedBytes::VALUE_ALIGNMENT
	#[inline]
	unsafe fn set_pos(&mut self, new_pos: usize) {
		debug_assert!(new_pos <= self.capacity);
		debug_assert!(is_aligned_to(new_pos, VALUE_ALIGNMENT));

		self.pos = new_pos;
	}

	/// Push a slice of values `&T` to storage, without alignment checks and
	/// without reserving capacity for it.
	///
	/// # Safety
	///
	/// Caller must ensure [`AlignedBytes`] has sufficient capacity.
	///
	/// `size` must be total size in bytes of `&[T]`.
	/// i.e. `size = mem::size_of::<T>() * slice.len()`.
	///
	/// This method does **not** ensure 2 invariants of storage relating to
	/// alignment:
	///
	/// * that [`pos()`] is aligned for the type before push.
	/// * that [`pos()`] is aligned to [`VALUE_ALIGNMENT`] after push.
	///
	/// Caller must uphold these invariants. It is sufficient to:
	///
	/// * call [`align_for::<T>()`](Storage::align_for) before and
	/// * call [`align_after::<T>()`](Storage::align_after) after.
	///
	/// [`pos()`]: Storage::pos
	/// [`VALUE_ALIGNMENT`]: Storage::VALUE_ALIGNMENT
	#[inline]
	unsafe fn push_slice_unchecked<T>(&mut self, slice: &[T], size: usize) {
		debug_assert!(self.capacity - self.pos >= size);
		debug_assert_eq!(size, mem::size_of::<T>() * slice.len());
		debug_assert!(is_aligned_to(self.pos, mem::align_of::<T>()));

		// Do nothing if ZST. This function will be compiled down to a no-op for ZSTs.
		if mem::size_of::<T>() == 0 {
			return;
		}

		self.write_slice(self.pos, slice);
		self.pos += size;
	}

	/// Ensure capacity for at least `additional` more bytes to be inserted into
	/// the [`AlignedBytes`].
	///
	/// # Panics
	///
	/// Panics if this reservation would cause [`AlignedBytes`] to exceed its
	/// capacity.
	#[inline]
	fn reserve(&mut self, additional: usize) {
		// Cannot wrap because capacity always exceeds pos,
		// but avoids having to handle potential overflow here
		let remaining = self.capacity.wrapping_sub(self.pos);
		if additional > remaining {
			self.over_capacity();
		}
	}
}

impl<
		const STORAGE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize,
	> AlignedBytes<STORAGE_ALIGNMENT, MAX_VALUE_ALIGNMENT, VALUE_ALIGNMENT, MAX_CAPACITY>
{
	/// Panic after `reserve` has found insufficient capacity for reservation
	/// request.
	///
	/// This is a separate function marked `#[cold]` to hint to compiler that this
	/// branch is not often taken. This keeps the path for common case where
	/// capacity is already sufficient as fast as possible, and makes `reserve`
	/// more likely to be inlined.
	/// This is the same trick that Rust's `Vec::reserve` uses.
	#[cold]
	fn over_capacity(&mut self) {
		panic!("Cannot grow AlignedBytes");
	}
}

impl<
		const STORAGE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize,
	> RandomAccessStorage
	for AlignedBytes<STORAGE_ALIGNMENT, MAX_VALUE_ALIGNMENT, VALUE_ALIGNMENT, MAX_CAPACITY>
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
	unsafe fn write_slice<T>(&mut self, pos: usize, slice: &[T]) {
		debug_assert!(pos <= self.capacity);
		debug_assert!(self.capacity - pos >= mem::size_of::<T>() * slice.len());
		debug_assert!(is_aligned_to(pos, mem::align_of::<T>()));

		// Do nothing if ZST. This function will be compiled down to a no-op for ZSTs.
		if mem::size_of::<T>() == 0 {
			return;
		}

		let src = slice.as_ptr();
		let dst = self.ptr.as_ptr().add(pos) as *mut T;
		// `src` must be correctly aligned as derived from a valid `&[T]`.
		// Ensuring sufficient capacity is a requirement of this method.
		// `dst` being correctly aligned is a requirement of this method.
		ptr::copy_nonoverlapping(src, dst, slice.len());
	}

	/// Get immutable reference for a value at a specific position in storage.
	///
	/// # Safety
	///
	/// * A `T` must be present at this position in the storage.
	/// * `pos` must be correctly aligned for `T`.
	unsafe fn read<T>(&self, pos: usize) -> &T {
		debug_assert!(pos + mem::size_of::<T>() <= self.pos);
		debug_assert!(is_aligned_to(pos, mem::align_of::<T>()));

		let ptr = self.ptr.as_ptr().add(pos) as *const T;
		&*ptr.cast()
	}

	/// Get mutable reference for a value at a specific position in storage.
	///
	/// # Safety
	///
	/// * A `T` must be present at this position in the storage.
	/// * `pos` must be correctly aligned for `T`.
	unsafe fn read_mut<T>(&mut self, pos: usize) -> &mut T {
		debug_assert!(pos + mem::size_of::<T>() <= self.pos);
		debug_assert!(is_aligned_to(pos, mem::align_of::<T>()));

		let ptr = self.ptr.as_ptr().add(pos) as *mut T;
		&mut *ptr.cast()
	}

	/// Returns a raw pointer to a position in the storage.
	///
	/// The caller must ensure that the storage outlives the pointer this function
	/// returns, or else it will end up pointing to garbage.
	///
	/// # Safety
	///
	/// * Storage must have allocated (i.e. initialized with [`with_capacity`], or
	///   have had some values pushed to it).
	/// * `pos` must be a valid position within the storage's allocation.
	///
	/// [`with_capacity`]: AlignedBytes::with_capacity
	#[inline]
	unsafe fn ptr(&self, pos: usize) -> *const u8 {
		debug_assert!(self.capacity > 0);
		debug_assert!(pos <= self.capacity);

		self.ptr.as_ptr().add(pos)
	}

	/// Returns an unsafe mutable pointer a position in the storage.
	///
	/// The caller must ensure that the storage outlives the pointer this function
	/// returns, or else it will end up pointing to garbage.
	///
	/// # Safety
	///
	/// * Storage must have allocated (i.e. initialized with [`with_capacity`], or
	///   have had some values pushed to it).
	/// * `pos` must be a valid position within the storage's allocation.
	///
	/// [`with_capacity`]: AlignedBytes::with_capacity
	#[inline]
	unsafe fn mut_ptr(&mut self, pos: usize) -> *mut u8 {
		debug_assert!(self.capacity > 0);
		debug_assert!(pos <= self.capacity);

		self.ptr.as_ptr().add(pos)
	}
}

impl<
		const STORAGE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize,
	> ContiguousStorage
	for AlignedBytes<STORAGE_ALIGNMENT, MAX_VALUE_ALIGNMENT, VALUE_ALIGNMENT, MAX_CAPACITY>
{
	/// Returns a raw pointer to the start of the storage's buffer, or a dangling
	/// raw pointer valid for zero sized reads if the storage didn't allocate.
	///
	/// The caller must ensure that the storage outlives the pointer this function
	/// returns, or else it will end up pointing to garbage.
	#[inline]
	fn as_ptr(&self) -> *const u8 {
		self.ptr.as_ptr()
	}

	/// Returns an unsafe mutable pointer to the start of the storage's buffer, or
	/// a dangling raw pointer valid for zero sized reads if the storage didn't
	/// allocate.
	///
	/// The caller must ensure that the storage outlives the pointer this function
	/// returns, or else it will end up pointing to garbage.
	#[inline]
	fn as_mut_ptr(&mut self) -> *mut u8 {
		self.ptr.as_ptr()
	}
}

/// `AlignedBytes` memory is fixed and does not move.
impl<
		const STORAGE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize,
	> PinnedStorage
	for AlignedBytes<STORAGE_ALIGNMENT, MAX_VALUE_ALIGNMENT, VALUE_ALIGNMENT, MAX_CAPACITY>
{
}

impl<
		const STORAGE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize,
	> Drop for AlignedBytes<STORAGE_ALIGNMENT, MAX_VALUE_ALIGNMENT, VALUE_ALIGNMENT, MAX_CAPACITY>
{
	#[inline]
	fn drop(&mut self) {
		if self.capacity > 0 {
			unsafe {
				alloc::dealloc(
					self.ptr.as_ptr(),
					Layout::from_size_align_unchecked(self.capacity, STORAGE_ALIGNMENT),
				)
			};
		}
	}
}

// Safe to be `Send` and `Sync` because pointer is not aliased and does not use
// interior mutability.
unsafe impl<
		const STORAGE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize,
	> Send for AlignedBytes<STORAGE_ALIGNMENT, MAX_VALUE_ALIGNMENT, VALUE_ALIGNMENT, MAX_CAPACITY>
{
}

unsafe impl<
		const STORAGE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_CAPACITY: usize,
	> Sync for AlignedBytes<STORAGE_ALIGNMENT, MAX_VALUE_ALIGNMENT, VALUE_ALIGNMENT, MAX_CAPACITY>
{
}
