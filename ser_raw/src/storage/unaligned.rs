use super::{ContiguousStorage, Storage};

/// Trait for storage used by Serializers which has no specified alignment in
/// memory.
pub trait UnalignedStorage: Storage {}

// TODO: Should use `Vec<MaybeUninit<u8>>` not `Vec<u8>` as output likely
// includes uninitialized padding bytes

/// Unaligned contiguous memory buffer. Used by `UnalignedSerializer`.
///
/// Just a wrapper around `Vec<u8>`.
pub struct UnalignedVec {
	inner: Vec<u8>,
}

impl UnalignedVec {
	/// Push bytes to storage.
	/// Not exposed in public API, but included for use in `UnalignedVec`.
	/// It's slightly faster than a hand-rolled `ptr::copy_nonoverlapping`-based
	/// version.
	#[inline]
	pub(crate) fn extend_from_slice(&mut self, bytes: &[u8]) {
		self.inner.extend_from_slice(bytes);
	}
}

impl Storage for UnalignedVec {
	/// Create new `UnalignedVec` without allocating any memory.
	#[inline]
	fn new() -> Self {
		Self { inner: Vec::new() }
	}

	/// Create new `UnalignedVec` with pre-allocated capacity.
	#[inline]
	fn with_capacity(capacity: usize) -> Self {
		Self {
			inner: Vec::with_capacity(capacity),
		}
	}

	/// Create new `UnalignedVec` with pre-allocated capacity.
	///
	/// For `UnalignedVec`, there is no advantage to this method over the safe
	/// method `with_capacity`. They both do exactly the same thing.
	///
	/// Prefer `with_capacity`.
	///
	/// Despite being an unsafe method, there are no invariants which must be
	/// satisfied. Method is unsafe purely because the trait method is unsafe,
	/// because other `Storage` types may impose safety requirements.
	#[inline]
	unsafe fn with_capacity_unchecked(capacity: usize) -> Self {
		Self {
			inner: Vec::with_capacity(capacity),
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

	/// Set amount of storage currently used.
	///
	/// # Safety
	///
	/// * `new_len` must be less than or equal `capacity()`.
	#[inline]
	unsafe fn set_len(&mut self, new_len: usize) {
		debug_assert!(new_len <= self.capacity());
		self.inner.set_len(new_len);
	}

	/// Reserve space in storage for `additional` bytes, growing capacity if
	/// required.
	#[inline]
	fn reserve(&mut self, additional: usize) {
		self.inner.reserve(additional);
	}

	/// Clear contents of storage.
	///
	/// Does not reduce the storage's capacity, just resets `len` back to 0.
	#[inline]
	fn clear(&mut self) {
		self.inner.clear();
	}

	/// Shrink the capacity of the storage as much as possible.
	#[inline]
	fn shrink_to_fit(&mut self) {
		self.inner.shrink_to_fit();
	}
}

impl ContiguousStorage for UnalignedVec {
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

impl UnalignedStorage for UnalignedVec {}
