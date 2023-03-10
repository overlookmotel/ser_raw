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
	#[inline]
	fn new() -> Self {
		Self { inner: Vec::new() }
	}

	#[inline]
	fn with_capacity(capacity: usize) -> Self {
		Self {
			inner: Vec::with_capacity(capacity),
		}
	}

	/// Create new `UnalignedVec` with specified capacity.
	///
	/// This method is unsafe, only because it is for the `Storage` trait -
	/// because other storage types do have safety invariants. But `UnalignedVec`
	/// has none.
	///
	/// There is no reason to use this method. Use the safe `with_capacity()`
	/// method instead, which does exactly the same thing.
	#[inline]
	unsafe fn with_capacity_unchecked(capacity: usize) -> Self {
		Self {
			inner: Vec::with_capacity(capacity),
		}
	}

	#[inline]
	fn capacity(&self) -> usize {
		self.inner.capacity()
	}

	#[inline]
	fn len(&self) -> usize {
		self.inner.len()
	}

	#[inline]
	unsafe fn set_len(&mut self, new_len: usize) {
		self.inner.set_len(new_len);
	}

	#[inline]
	fn reserve(&mut self, additional: usize) {
		self.inner.reserve(additional);
	}

	#[inline]
	fn clear(&mut self) {
		self.inner.clear();
	}

	#[inline]
	fn shrink_to_fit(&mut self) {
		self.inner.shrink_to_fit();
	}
}

impl ContiguousStorage for UnalignedVec {
	#[inline]
	fn as_ptr(&self) -> *const u8 {
		self.inner.as_ptr()
	}

	#[inline]
	fn as_mut_ptr(&mut self) -> *mut u8 {
		self.inner.as_mut_ptr()
	}

	#[inline]
	fn as_slice(&self) -> &[u8] {
		self.inner.as_slice()
	}

	#[inline]
	fn as_mut_slice(&mut self) -> &mut [u8] {
		self.inner.as_mut_slice()
	}
}

impl UnalignedStorage for UnalignedVec {}
