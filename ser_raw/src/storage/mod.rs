use std::{mem, slice};

mod aligned;
pub use aligned::{AlignedStorage, AlignedVec};

mod unaligned;
pub use unaligned::{UnalignedStorage, UnalignedVec};

mod aligned_vec;
pub(crate) use aligned_vec::AlignedByteVec;

// TODO: Add `MAX_CAPACITY` const param
// TODO: Add `push_empty` method

/// Trait for storage used by Serializers.
///
/// Types implementing `Storage` are usually simple wrappers around another data
/// structure (e.g. `Vec<u8>`), but `Storage` provides a more constrained API,
/// so `Storage` types can enforce invariants about how storage is structured.
pub trait Storage {
	/// Create new `Storage` instance.
	fn new() -> Self;

	/// Create new `Storage` instance with pre-allocated capacity.
	fn with_capacity(capacity: usize) -> Self;

	/// Create new `Storage` instance with pre-allocated capacity,
	/// without safety checks.
	///
	/// # Safety
	///
	/// This trait imposes no safety requirements of its own, but individual
	/// `Storage` types may do.
	unsafe fn with_capacity_unchecked(capacity: usize) -> Self;

	/// Returns current capacity of storage in bytes.
	fn capacity(&self) -> usize;

	/// Returns amount of storage currently used in bytes.
	fn len(&self) -> usize;

	/// Set amount of storage currently used.
	///
	/// # Safety
	///
	/// * `new_len` must be less than or equal `capacity()`.
	///
	/// Storage types may impose additional safety requirements.
	unsafe fn set_len(&mut self, new_len: usize) -> ();

	/// Push a value of type `T` to storage.
	#[inline]
	fn push<T>(&mut self, value: &T) {
		self.push_slice(slice::from_ref(value));
	}

	/// Push a slice of values `&T` to storage.
	#[inline]
	fn push_slice<T>(&mut self, slice: &[T]) {
		self.align_for::<T>();
		// `push_slice_unaligned`'s requirements are satisfied by `align_for::<T>()` and
		// `align_after::<T>()`
		unsafe { self.push_slice_unaligned(slice) };
		self.align_after::<T>();
	}

	/// Push a slice of raw bytes to storage.
	#[inline]
	fn push_bytes(&mut self, bytes: &[u8]) {
		self.push_slice(bytes);
	}

	/// Push a slice of values `&T` to storage, without ensuring alignment first.
	///
	/// # Safety
	///
	/// Some `Storage` types may impose requirements concerning alignment which
	/// caller must satisfy.
	///
	/// Implementations must ensure that to satisfy these requirements, it's
	/// sufficient to:
	///
	/// * call `align_for::<T>()` before and
	/// * call `align_after::<T>()` after.
	#[inline]
	unsafe fn push_slice_unaligned<T>(&mut self, slice: &[T]) {
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
	/// Caller must ensure `Storage` has sufficient capacity.
	///
	/// `size` must be total size in bytes of `&[T]`.
	/// i.e. `size = mem::size_of::<T>() * slice.len()`.
	///
	/// Some `Storage` types may impose requirements concerning alignment which
	/// caller must satisfy.
	///
	/// Implementations must ensure that to satisfy any alignment requirements,
	/// it's sufficient to:
	///
	/// * call `align_for::<T>()` before and
	/// * call `align_after::<T>()` after.
	unsafe fn push_slice_unchecked<T>(&mut self, slice: &[T], size: usize) -> ();

	/// Reserve space in storage for `additional` bytes, growing capacity if
	/// required.
	fn reserve(&mut self, additional: usize) -> ();

	/// Align position in storage to alignment of `T`.
	/// Should be called before calling `push_slice_unaligned`.
	fn align_for<T>(&mut self) -> ();

	/// Align position in storage after pushing a `T` or slice `&[T]` with
	/// `push_slice_unaligned`.
	fn align_after<T>(&mut self) -> ();

	/// Align position in storage after pushing values of any type with
	/// `push_slice_unaligned`.
	///
	/// `align_after<T>` is often more efficient and can often be compiled down to
	/// a no-op, so is preferred.
	fn align_after_any(&mut self) -> ();

	/// Align position in storage to `alignment`.
	///
	/// # Safety
	///
	/// * `alignment` must be less than `isize::MAX`.
	/// * `alignment` must be a power of 2.
	///
	/// Some `Storage` types may impose additional safety requirements.
	unsafe fn align(&mut self, alignment: usize) -> ();

	/// Clear contents of storage.
	///
	/// Does not reduce the storage's capacity, just resets `len` back to 0.
	fn clear(&mut self) -> ();

	/// Shrink the capacity of the storage as much as possible.
	fn shrink_to_fit(&mut self) -> ();
}

// TODO: Add `write` + `write_unchecked` methods to `ContiguousStorage`

/// Trait for storage used by Serializers which store data in a contiguous
/// memory region.
pub trait ContiguousStorage: Storage {
	/// Returns a raw pointer to the storage's buffer, or a dangling raw pointer
	/// valid for zero sized reads if the storage didn't allocate.
	///
	/// The caller must ensure that the storage outlives the pointer this function
	/// returns, or else it will end up pointing to garbage. Modifying the storage
	/// may cause its buffer to be reallocated, which would also make any pointers
	/// to it invalid.
	fn as_ptr(&self) -> *const u8;

	/// Returns an unsafe mutable pointer to the storage's buffer, or a dangling
	/// raw pointer valid for zero sized reads if the storage didn't allocate.
	///
	/// The caller must ensure that the storage outlives the pointer this function
	/// returns, or else it will end up pointing to garbage. Modifying the storage
	/// may cause its buffer to be reallocated, which would also make any pointers
	/// to it invalid.
	fn as_mut_ptr(&mut self) -> *mut u8;

	/// Extracts a slice containing the entire storage buffer.
	fn as_slice(&self) -> &[u8];

	/// Extracts a mutable slice of the entire storage buffer.
	fn as_mut_slice(&mut self) -> &mut [u8];
}
