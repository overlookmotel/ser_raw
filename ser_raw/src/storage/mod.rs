//! Storage types and traits.

use std::{marker::PhantomData, mem, slice};

use crate::util::{align_up_to, aligned_max_capacity, is_aligned_to};

mod aligned_vec;
pub use aligned_vec::AlignedVec;

/// Trait for storage used by [`Serializer`]s which ensures values added to
/// storage maintain correct alignment in memory for their types.
///
/// [`AlignedVec`] implements this trait. You could also build your own
/// implementation of [`Storage`] with different properties.
///
/// # Const parameters
///
/// By configuring alignment requirements statically, the compiler is able to
/// remove alignment calculations for many cases. This improves performance.
///
/// ## `STORAGE_ALIGNMENT`
///
/// Alignment of the underlying memory used by the [`Storage`].
///
/// * Must be a power of 2.
///
/// Default: 16
///
/// ## `MAX_VALUE_ALIGNMENT`
///
/// Maximum alignment requirement of values which can be stored in the
/// [`Storage`]. Types with alignment greater than [`MAX_VALUE_ALIGNMENT`]
/// cannot be stored in this [`Storage`].
///
/// * Must be a power of 2.
/// * Must be less than or equal to [`STORAGE_ALIGNMENT`].
///
/// [`capacity()`](Storage::capacity) will always be a multiple of this.
///
/// Default: [`STORAGE_ALIGNMENT`]
///
/// ## `VALUE_ALIGNMENT`
///
/// Minimum alignment all values will have in the [`Storage`].
///
/// Types with alignment higher than [`VALUE_ALIGNMENT`] will have padding
/// inserted before them if required. Types with alignment lower than
/// [`VALUE_ALIGNMENT`] will have padding inserted after them to leave the
/// [`Storage`] aligned on [`VALUE_ALIGNMENT`], ready for the next
/// [`push()`](Storage::push).
///
/// This doesn't affect the "legality" of the output, but if most allocated
/// types being serialized have the same alignment, setting [`VALUE_ALIGNMENT`]
/// to that alignment may significantly improve performance, as alignment
/// calculations can be skipped when serializing those types.
///
/// NB: The word "allocated" in "allocated types" is key here. `ser_raw` deals
/// in allocations, not individual types. So this means that only types which
/// are pointed to by a `Box<T>` or `Vec<T>` count as "allocated types"
/// for the purposes of calculating an optimal value for [`VALUE_ALIGNMENT`].
///
/// e.g. If all (or almost all) types contain pointers (`Box`, `Vec` etc),
/// setting `VALUE_ALIGNMENT = std::mem::size_of::<usize>()`
/// will be the best value for fast serialization.
///
/// The higher [`VALUE_ALIGNMENT`] is, the more padding bytes will be added to
/// output, potentially increasing output size - though the degree of that
/// effect depends on the types being serialized.
///
/// * Must be a power of 2.
/// * Must be less than or equal to [`MAX_VALUE_ALIGNMENT`].
///
/// Default:
///
/// * 64-bit systems: 8
/// * 32-bit systems: 4
///
/// ## `MAX_CAPACITY`
///
/// Maximum capacity of the [`Storage`].
///
/// * Cannot be 0.
/// * Cannot be greater than `isize::MAX + 1 - STORAGE_ALIGNMENT`.
/// * Must be a multiple of [`MAX_VALUE_ALIGNMENT`].
///
/// Default:
///
/// * 64-bit systems: `i64::MAX - 15`
/// * 32-bit systems: `i32::MAX - 15`
///
/// # Implementing `Storage`
///
/// [`Storage`] trait provides default methods which implement the alignment
/// logic. To implement a [`Storage`], you'd only need to implement the methods
/// which don't have a default method already provided.
///
/// All the `push*` methods eventually lead to [`reserve`](Storage::reserve) and
/// [`push_slice_unchecked`](Storage::push_slice_unchecked), so only those two
/// need to be implemented to customize the central part of [`Storage`]'s
/// operation.
///
/// [`new`](Storage::new), [`with_capacity`](Storage::with_capacity), and
/// [`with_capacity_unchecked`](Storage::with_capacity_unchecked) should include
/// this line, to validate the const parameters at compile time:
///
/// ```ignore
/// let _ = Self::ASSERT_ALIGNMENTS_VALID;
/// ```
///
/// [`Serializer`]: crate::Serializer
/// [`STORAGE_ALIGNMENT`]: Storage::STORAGE_ALIGNMENT
/// [`MAX_VALUE_ALIGNMENT`]: Storage::MAX_VALUE_ALIGNMENT
/// [`VALUE_ALIGNMENT`]: Storage::VALUE_ALIGNMENT
pub trait Storage: Sized {
	/// Alignment of storage's memory buffer.
	///
	/// See [`Storage`] trait for explanation.
	const STORAGE_ALIGNMENT: usize;

	/// Maximum alignment of values being added to storage.
	///
	/// See [`Storage`] trait for explanation.
	const MAX_VALUE_ALIGNMENT: usize;

	/// Typical alignment of values being added to storage.
	///
	/// See [`Storage`] trait for explanation.
	const VALUE_ALIGNMENT: usize;

	/// Maximum capacity of storage.
	///
	/// See [`Storage`] trait for explanation.
	const MAX_CAPACITY: usize;

	/// Assertions for validity of alignment const parameters.
	/// These assertions are not evaluated just by this const param being present.
	///
	/// `Self::ASSERT_ALIGNMENTS_VALID` must be referenced in all code paths
	/// creating a `Storage`, to produce a compile-time error if assertions fail.
	const ASSERT_ALIGNMENTS_VALID: () = {
		assert!(Self::STORAGE_ALIGNMENT > 0, "STORAGE_ALIGNMENT cannot be 0");
		assert!(
			Self::STORAGE_ALIGNMENT < isize::MAX as usize,
			"STORAGE_ALIGNMENT must be less than isize::MAX"
		);
		assert!(
			Self::STORAGE_ALIGNMENT.is_power_of_two(),
			"STORAGE_ALIGNMENT must be a power of 2"
		);

		assert!(
			Self::MAX_VALUE_ALIGNMENT > 0,
			"MAX_VALUE_ALIGNMENT cannot be 0"
		);
		assert!(
			Self::MAX_VALUE_ALIGNMENT <= Self::STORAGE_ALIGNMENT,
			"MAX_VALUE_ALIGNMENT must be less than or equal to STORAGE_ALIGNMENT",
		);
		assert!(
			Self::MAX_VALUE_ALIGNMENT.is_power_of_two(),
			"MAX_VALUE_ALIGNMENT must be a power of 2"
		);

		assert!(Self::VALUE_ALIGNMENT > 0, "VALUE_ALIGNMENT cannot be 0");
		assert!(
			Self::VALUE_ALIGNMENT <= Self::MAX_VALUE_ALIGNMENT,
			"VALUE_ALIGNMENT must be less than or equal to MAX_VALUE_ALIGNMENT",
		);
		assert!(
			Self::VALUE_ALIGNMENT.is_power_of_two(),
			"VALUE_ALIGNMENT must be a power of 2"
		);

		assert!(Self::MAX_CAPACITY > 0, "MAX_CAPACITY cannot be 0");
		assert!(
			Self::MAX_CAPACITY <= aligned_max_capacity(Self::STORAGE_ALIGNMENT),
			"MAX_CAPACITY cannot exceed isize::MAX + 1 - STORAGE_ALIGNMENT"
		);
		assert!(
			Self::MAX_CAPACITY % Self::MAX_VALUE_ALIGNMENT == 0,
			"MAX_CAPACITY must be a multiple of MAX_VALUE_ALIGNMENT"
		);
	};

	/// Create new `Storage` instance with no pre-allocated capacity.
	fn new() -> Self;

	/// Create new [`Storage`] with pre-allocated capacity.
	///
	/// Capacity will be rounded up to a multiple of
	/// [`MAX_VALUE_ALIGNMENT`](Storage::MAX_VALUE_ALIGNMENT).
	///
	/// # Panics
	///
	/// Panics if `capacity` exceeds [`MAX_CAPACITY`](Storage::MAX_CAPACITY).
	fn with_capacity(capacity: usize) -> Self {
		// Ensure (at compile time) that const params are valid
		let _ = Self::ASSERT_ALIGNMENTS_VALID;

		if capacity == 0 {
			return Self::new();
		}

		// Round up capacity to multiple of `MAX_VALUE_ALIGNMENT`.
		// Assertion ensures overflow in `align_up_to()` is not possible.
		assert!(
			capacity <= Self::MAX_CAPACITY,
			"capacity cannot exceed MAX_CAPACITY"
		);
		let capacity = align_up_to(capacity, Self::MAX_VALUE_ALIGNMENT);

		// Above assertion and `align_up_to` call satisfy `with_capacity_unchecked`'s
		// requirements
		unsafe { Self::with_capacity_unchecked(capacity) }
	}

	/// Create new `Storage` instance with pre-allocated capacity,
	/// without safety checks.
	///
	/// # Safety
	///
	/// * `capacity` must not be 0.
	/// * `capacity` must be less than or equal to
	///   [`MAX_CAPACITY`](Storage::MAX_CAPACITY).
	/// * `capacity` must be a multiple of
	///   [`MAX_VALUE_ALIGNMENT`](Storage::MAX_VALUE_ALIGNMENT).
	unsafe fn with_capacity_unchecked(capacity: usize) -> Self;

	/// Returns current capacity of storage in bytes.
	fn capacity(&self) -> usize;

	/// Returns current position in storage.
	fn pos(&self) -> usize;

	/// Set current position in storage.
	///
	/// # Safety
	///
	/// * `new_pos` must be less than or equal [`capacity()`](Storage::capacity).
	/// * `new_pos` must be a multiple of [`VALUE_ALIGNMENT`].
	///
	/// [`Storage`] implementations can impose further constraints.
	///
	/// [`VALUE_ALIGNMENT`]: Storage::VALUE_ALIGNMENT
	unsafe fn set_pos(&mut self, new_pos: usize) -> ();

	/// Push a value of type `T` to storage.
	#[inline]
	fn push<T>(&mut self, value: &T) {
		self.push_slice(slice::from_ref(value));
	}

	/// Push a slice of values `&T` to storage.
	///
	/// If the size of the slice is known statically, prefer `push<[T; N]>` to
	/// `push_slice<T>`, as the former is slightly more efficient.
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
	/// # Panics
	///
	/// Panics if would require growing storage beyond [`MAX_CAPACITY`].
	///
	/// # Safety
	///
	/// This method does **not** ensure 2 invariants relating to alignment:
	///
	/// * [`pos()`] must be aligned for the type before push.
	/// * [`pos()`] must be aligned to [`VALUE_ALIGNMENT`] after push.
	///
	/// Caller must uphold these invariants. It is sufficient to:
	///
	/// * call [`align_for::<T>()`](Storage::align_for) before and
	/// * call [`align_after::<T>()`](Storage::align_after) after.
	///
	/// [`Storage`] implementations must ensure that alignment requirements can be
	/// satisfied by the above.
	///
	/// [`pos()`]: Storage::pos
	/// [`VALUE_ALIGNMENT`]: Storage::VALUE_ALIGNMENT
	/// [`MAX_CAPACITY`]: Storage::MAX_CAPACITY
	#[inline]
	unsafe fn push_slice_unaligned<T>(&mut self, slice: &[T]) {
		debug_assert!(is_aligned_to(self.pos(), mem::align_of::<T>()));

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
	/// Caller must ensure [`Storage`] has sufficient capacity.
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
	/// [`Storage`] implementations must ensure that alignment requirements can be
	/// satisfied by the above.
	///
	/// [`pos()`]: Storage::pos
	/// [`VALUE_ALIGNMENT`]: Storage::VALUE_ALIGNMENT
	unsafe fn push_slice_unchecked<T>(&mut self, slice: &[T], size: usize) -> ();

	/// Advance buffer position to leave space to write a `T` at current position
	/// later.
	///
	/// Will also insert padding as required, to ensure the `T` can be written
	/// with correct alignment.
	#[inline]
	fn push_empty<T>(&mut self) {
		self.push_empty_slice::<T>(1);
	}

	/// Advance buffer position to leave space to write a slice `&[T]`
	/// (`T` x `len`) at current position later.
	///
	/// Will also insert padding as required, to ensure the `&[T]` can be written
	/// with correct alignment.
	///
	/// If the size of the slice is known statically, prefer
	/// `push_empty::<[T; N]>()` to `push_empty_slice::<T>(N)`,
	/// as the former is slightly more efficient.
	#[inline]
	fn push_empty_slice<T>(&mut self, len: usize) {
		self.align_for::<T>();

		let size = mem::size_of::<T>() * len;
		self.reserve(size);
		unsafe { self.set_pos(self.pos() + size) };

		self.align_after::<T>();
	}

	/// Reserve space in storage for `additional` bytes, growing capacity if
	/// required.
	///
	/// # Panics
	///
	/// Panics if this reservation would cause the [`Storage`] to exceed
	/// [`MAX_CAPACITY`](Storage::MAX_CAPACITY).
	fn reserve(&mut self, additional: usize) -> ();

	/// Align position in storage to alignment of `T`.
	///
	/// Should be called before calling
	/// [`push_slice_unaligned`](Storage::push_slice_unaligned).
	#[inline(always)] // Because this is generally a no-op
	fn align_for<T>(&mut self) {
		// Ensure (at compile time) that `T`'s alignment does not exceed
		// `MAX_VALUE_ALIGNMENT`
		let _ = AlignmentCheck::<T, Self>::ASSERT_ALIGNMENT_DOES_NOT_EXCEED;

		// Align position in output buffer to alignment of `T`.
		// If `T`'s alignment requirement is less than or equal to `VALUE_ALIGNMENT`,
		// this can be skipped, as position is always left aligned to `VALUE_ALIGNMENT`
		// after each push.
		// This should be optimized away for types with alignment of `VALUE_ALIGNMENT`
		// or less, in which case this function becomes a no-op.
		// Hopefully this is the majority of types.
		if mem::align_of::<T>() > Self::VALUE_ALIGNMENT {
			// Static assertion above ensures `align()`'s constraints are satisfied
			unsafe { self.align(mem::align_of::<T>()) }
		}
	}

	/// Align position in storage after pushing a `T` or slice `&[T]` with
	/// [`push_slice_unaligned`](Storage::push_slice_unaligned).
	#[inline(always)] // Because this is generally a no-op
	fn align_after<T>(&mut self) {
		// Align buffer position to `VALUE_ALIGNMENT`, ready for the next value.
		// This should be optimized away for types with alignment of `VALUE_ALIGNMENT`
		// or greater. Ditto for types which have lower alignment, but happen to have
		// size divisible by `VALUE_ALIGNMENT`. Hopefully this is the majority of types.
		if mem::size_of::<T>() % Self::VALUE_ALIGNMENT > 0 {
			self.align_after_any();
		}
	}

	/// Align position in storage after pushing values of any type with
	/// [`push_slice_unaligned`](Storage::push_slice_unaligned).
	///
	/// [`align_after<T>()`](Storage::align_after) is often a better choice as it
	/// can often be compiled down to a no-op.
	#[inline]
	fn align_after_any(&mut self) {
		// `VALUE_ALIGNMENT` trivially fulfills `align()`'s requirements
		unsafe { self.align(Self::VALUE_ALIGNMENT) };
	}

	/// Align position in storage to `alignment`.
	///
	/// # Safety
	///
	/// The following constraints must be satisfied in order to leave the
	/// [`Storage`] in a consistent state:
	///
	/// * `alignment` must be >= [`VALUE_ALIGNMENT`](Storage::VALUE_ALIGNMENT).
	/// * `alignment` must be <=
	///   [`MAX_VALUE_ALIGNMENT`](Storage::MAX_VALUE_ALIGNMENT).
	///
	/// For alignment calculation to be valid:
	///
	/// * `alignment` must be a power of 2.
	#[inline]
	unsafe fn align(&mut self, alignment: usize) {
		debug_assert!(alignment >= Self::VALUE_ALIGNMENT);
		debug_assert!(alignment <= Self::MAX_VALUE_ALIGNMENT);
		debug_assert!(alignment.is_power_of_two());

		// Round up buffer position to multiple of `alignment`.
		// `align_up_to`'s constraints are satisfied by:
		// * `self.pos` is always less than `MAX_CAPACITY`, which is `< isize::MAX`.
		// * `alignment <= MAX_VALUE_ALIGNMENT` satisfies `alignment < isize::MAX`
		//   because `MAX_VALUE_ALIGNMENT < isize::MAX`.
		// * `alignment` being a power of 2 is part of this function's contract.
		let new_pos = align_up_to(self.pos(), alignment);

		// `new_pos > capacity` can't happen because of 2 guarantees:
		// 1. `alignment <= MAX_VALUE_ALIGNMENT`
		// 2. `capacity` is a multiple of `MAX_VALUE_ALIGNMENT`
		self.set_pos(new_pos);
	}

	/// Clear contents of storage.
	///
	/// Does not reduce the storage's capacity, just resets
	/// [`pos()`](Storage::pos) back to 0.
	#[inline]
	fn clear(&mut self) {
		// 0 trivially satisfies requirement that `new_pos < self.capacity()`
		unsafe { self.set_pos(0) };
	}

	/// Shrink the capacity of the storage as much as possible.
	fn shrink_to_fit(&mut self) -> ();
}

/// Trait for [`Storage`] which supports random access read and writes.
///
/// [`Serializer`]: crate::Serializer
pub trait RandomAccessStorage: Storage {
	/// Write a value at a specific position in storage's buffer.
	///
	/// # Safety
	///
	/// Storage [`capacity()`] must be greater or equal to
	/// `pos + std::mem::size_of::<T>()`.
	/// i.e. write is within storage's allocation.
	///
	/// `pos` must be correctly aligned for `T`.
	///
	/// [`capacity()`]: Storage::capacity
	#[inline]
	unsafe fn write<T>(&mut self, pos: usize, value: &T) {
		self.write_slice(pos, slice::from_ref(value));
	}

	/// Write a slice of values at a specific position in storage's buffer.
	///
	/// # Safety
	///
	/// Storage [`capacity()`] must be greater or equal to
	/// `pos + std::mem::size_of::<T>() * slice.len()`.
	/// i.e. write is within storage's allocation.
	///
	/// `pos` must be correctly aligned for `T`.
	///
	/// [`capacity()`]: Storage::capacity
	unsafe fn write_slice<T>(&mut self, pos: usize, slice: &[T]) -> ();

	/// Read a value at a specific position in storage.
	///
	/// Returns an owned `T`. `T` must be `Copy`.
	///
	/// # Safety
	///
	/// * A `T` must be present at this position in the storage.
	/// * `pos` must be correctly aligned for `T`.
	#[inline]
	unsafe fn read<T: Copy>(&self, pos: usize) -> T {
		*self.read_ref(pos)
	}

	/// Get immutable reference for a value at a specific position in storage.
	///
	/// # Safety
	///
	/// * A `T` must be present at this position in the storage.
	/// * `pos` must be correctly aligned for `T`.
	unsafe fn read_ref<T>(&self, pos: usize) -> &T;

	/// Get mutable reference for a value at a specific position in storage.
	///
	/// # Safety
	///
	/// * A `T` must be present at this position in the storage.
	/// * `pos` must be correctly aligned for `T`.
	unsafe fn read_mut<T>(&mut self, pos: usize) -> &mut T;

	/// Returns a raw pointer to a position in the storage.
	///
	/// The caller must ensure that the storage outlives the pointer this function
	/// returns, or else it will end up pointing to garbage. Modifying the storage
	/// may cause its buffer to be reallocated, which would also make any pointers
	/// to it invalid.
	///
	/// # Safety
	///
	/// * Storage must have allocated (i.e. initialized with [`with_capacity`], or
	///   have had some values pushed to it).
	/// * `pos` must be a valid position within the storage's allocation.
	///
	/// [`with_capacity`]: Storage::with_capacity
	unsafe fn ptr(&self, pos: usize) -> *const u8;

	/// Returns an unsafe mutable pointer a position in the storage.
	///
	/// The caller must ensure that the storage outlives the pointer this function
	/// returns, or else it will end up pointing to garbage. Modifying the storage
	/// may cause its buffer to be reallocated, which would also make any pointers
	/// to it invalid.
	///
	/// # Safety
	///
	/// * Storage must have allocated (i.e. initialized with [`with_capacity`], or
	///   have had some values pushed to it).
	/// * `pos` must be a valid position within the storage's allocation.
	///
	/// [`with_capacity`]: Storage::with_capacity
	unsafe fn mut_ptr(&mut self, pos: usize) -> *mut u8;
}

/// Trait for [`Storage`] which stores data in a contiguous memory region.
pub trait ContiguousStorage: Storage {
	/// Returns a raw pointer to the start of the storage's buffer, or a dangling
	/// raw pointer valid for zero sized reads if the storage didn't allocate.
	///
	/// The caller must ensure that the storage outlives the pointer this function
	/// returns, or else it will end up pointing to garbage. Modifying the storage
	/// may cause its buffer to be reallocated, which would also make any pointers
	/// to it invalid.
	fn as_ptr(&self) -> *const u8;

	/// Returns an unsafe mutable pointer to the start of the storage's buffer, or
	/// a dangling raw pointer valid for zero sized reads if the storage didn't
	/// allocate.
	///
	/// The caller must ensure that the storage outlives the pointer this function
	/// returns, or else it will end up pointing to garbage. Modifying the storage
	/// may cause its buffer to be reallocated, which would also make any pointers
	/// to it invalid.
	fn as_mut_ptr(&mut self) -> *mut u8;
}

/// Type for static assertion that types being serialized do not have a higher
/// alignment requirement than the alignment of the output buffer
pub(crate) struct AlignmentCheck<T, S: Storage> {
	_marker: PhantomData<T>,
	_marker2: PhantomData<S>,
}

impl<T, S: Storage> AlignmentCheck<T, S> {
	pub const ASSERT_ALIGNMENT_DOES_NOT_EXCEED: () =
		assert!(mem::align_of::<T>() <= S::MAX_VALUE_ALIGNMENT);
}
