mod aligned;
pub use aligned::{AlignedStorage, AlignedVec};

mod unaligned;
pub use unaligned::{UnalignedStorage, UnalignedVec};

mod aligned_vec;
pub(crate) use aligned_vec::AlignedByteVec;

/// Trait for storage used by Serializers.
///
/// Types implementing `Storage` are usually simple wrappers around another data
/// structure (e.g. `Vec<u8>`), but `Storage` provides a more constrained API,
/// so `Storage` types can enforce invariants about how storage is structured.
pub trait Storage {
	/// Create new `Storage` instance
	fn new() -> Self;

	/// Create new `Storage` instance with pre-allocated capacity
	fn with_capacity(capacity: usize) -> Self;

	/// Create new `Storage` instance with pre-allocated capacity,
	/// without safety checks.
	///
	/// # Safety
	///
	/// This trait imposes no constraints of its own, but individual `Storage`
	/// types may do.
	unsafe fn with_capacity_unchecked(capacity: usize) -> Self;

	/// Returns current capacity of storage in bytes.
	fn capacity(&self) -> usize;

	/// Returns amount of storage currently used in bytes.
	fn len(&self) -> usize;

	/// Set amount of storage currently used.
	///
	/// # Safety
	///
	/// * `new_len` must be equal or less than `capacity()`.
	///
	/// Storage types may impose additional safety invariants.
	unsafe fn set_len(&mut self, new_len: usize) -> ();

	/// Reserve space in storage for `additional` bytes, growing capacity if
	/// required.
	fn reserve(&mut self, additional: usize) -> ();

	/// Clear contents of storage.
	///
	/// Does not reduce the storage's capacity, just resets `length` back to 0.
	fn clear(&mut self) -> ();

	/// Shrink the capacity of the storage as much as possible.
	fn shrink_to_fit(&mut self) -> ();
}

/// Trait for storage used by Serializers which stores data in a contiguous
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
