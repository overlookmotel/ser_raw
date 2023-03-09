use std::{borrow::BorrowMut, marker::PhantomData, mem, ptr};

use crate::{AlignedByteVec, Serializer};

/// Serializer that ensures values are correctly aligned in output buffer.
///
/// `OUTPUT_ALIGNMENT` is the alignment of the output buffer.
/// Types with alignment greater than `OUTPUT_ALIGNMENT` cannot be serialized
/// with this serializer.
///
/// `VALUE_ALIGNMENT` is minimum alignment all allocated values will have in
/// output buffer. Types with alignment higher than `VALUE_ALIGNMENT` will have
/// padding inserted before them if required. Types with alignment lower than
/// `VALUE_ALIGNMENT` will have padding inserted after to leave the buffer
/// aligned on `VALUE_ALIGNMENT` for the next insertion.
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
pub struct BaseSerializer<
	Buf: BorrowMut<AlignedByteVec<OUTPUT_ALIGNMENT>>,
	const OUTPUT_ALIGNMENT: usize,
	const VALUE_ALIGNMENT: usize,
	const MAX_VALUE_ALIGNMENT: usize,
> {
	buf: Buf,
}

impl<
		const OUTPUT_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
	>
	BaseSerializer<
		AlignedByteVec<OUTPUT_ALIGNMENT>,
		OUTPUT_ALIGNMENT,
		VALUE_ALIGNMENT,
		MAX_VALUE_ALIGNMENT,
	>
{
	/// Create new Serializer with minimal memory pre-allocated.
	pub fn new() -> Self {
		// Cannot start with 0 capacity as need to maintain invariant
		// `capacity % MAX_VALUE_ALIGNMENT == 0` throughout.
		// `align()` and `align_to_value_alignment()` rely on this invariant to prevent
		// setting `len` to more than `capacity`, without having to check each time.
		//
		// If capacity started at 0, this invariant could be broken as follows:
		// * `capacity = 0`
		// * `VALUE_ALIGNMENT = 8`
		// * `MAX_VALUE_ALIGNMENT = 8`
		// * 1st call to `push_slice_raw()` is with `&[0u8; 1]` (size 1, alignment 1)
		// * `push_slice_raw()` calls `AlignedByteVec::reserve(1)`.
		// * `push_slice_raw()` aligns position to `VALUE_ALIGNMENT` (8)
		// * Now `capacity = 1` and `len = 8`

		// `MAX_VALUE_ALIGNMENT` trivially fulfills `with_capacity_unchecked`'s contract
		unsafe { Self::with_capacity_unchecked(Self::MAX_VALUE_ALIGNMENT) }
	}

	/// Create new Serializer with buffer pre-allocated with capacity of
	/// at least `capacity` bytes.
	///
	/// `capacity` will be rounded up to a multiple of `MAX_VALUE_ALIGNMENT`.
	/// `capacity` cannot be 0.
	///
	/// If you know, or can estimate, the amount of buffer space that's going to
	/// be needed in advance, allocating upfront with `with_capacity` can
	/// dramatically improve performance vs using `new`.
	///
	/// # Panics
	///
	/// Panics if `capacity` is 0, or exceeds `MAX_CAPACITY`.
	pub fn with_capacity(capacity: usize) -> Self {
		// Bounds check `capacity`
		assert!(capacity != 0, "`capacity` cannot be 0");
		assert!(
			capacity <= Self::MAX_CAPACITY,
			"`capacity` cannot exceed `isize::MAX + 1 - OUTPUT_ALIGNMENT`"
		);

		// Round up capacity to multiple of `MAX_VALUE_ALIGNMENT`.
		// `AlignedByteVec`'s growth strategy is to double in size, so the invariant
		// that `capacity % MAX_VALUE_ALIGNMENT == 0` will always be maintained after
		// this starting point.
		// Above assertions + static assertions for allowable values
		// of `MAX_VALUE_ALIGNMENT` satisify constraints of `align_up_to`.
		let capacity = align_up_to(capacity, Self::MAX_VALUE_ALIGNMENT);

		// Above ensures compliance with `with_capacity_unchecked`'s contract
		unsafe { Self::with_capacity_unchecked(capacity) }
	}

	/// Create new Serializer with buffer pre-allocated with capacity of
	/// exactly `capacity` bytes.
	///
	/// # Panics
	///
	/// Panics if `capacity` exceeds `MAX_CAPACITY`.
	///
	/// # Safety
	///
	/// * `capacity` cannot be 0.
	/// * `capacity` must be a multiple of `MAX_VALUE_ALIGNMENT`.
	///
	/// Failure to obey these constraints may not produce UB immediately,
	/// but breaks assumptions the rest of the implementation relies on,
	/// so could cause arithmetic overflow or misaligned writes later on.
	pub unsafe fn with_capacity_unchecked(capacity: usize) -> Self {
		// Ensure (at compile time) that const params for alignment are valid
		let _ = Self::ASSERT_ALIGNMENTS_VALID;

		debug_assert!(capacity > 0);
		debug_assert!(capacity <= Self::MAX_CAPACITY);
		debug_assert!(is_aligned_to(capacity, Self::MAX_VALUE_ALIGNMENT));

		// `AlignedByteVec::with_capacity` panics if `capacity > MAX_CAPACITY`
		Self {
			buf: AlignedByteVec::with_capacity(capacity),
		}
	}
}

impl<
		Buf: BorrowMut<AlignedByteVec<OUTPUT_ALIGNMENT>>,
		const OUTPUT_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
	> BaseSerializer<Buf, OUTPUT_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT>
{
	/// Alignment of output buffer
	pub const OUTPUT_ALIGNMENT: usize = OUTPUT_ALIGNMENT;

	/// Typical alignment of values being serialized
	pub const VALUE_ALIGNMENT: usize = VALUE_ALIGNMENT;

	/// Maximum alignment of values being serialized
	pub const MAX_VALUE_ALIGNMENT: usize = MAX_VALUE_ALIGNMENT;

	/// Maximum capacity of output buffer.
	/// Dictated by the requirements of
	/// [`alloc::Layout`](https://doc.rust-lang.org/alloc/alloc/struct.Layout.html).
	/// "`size`, when rounded up to the nearest multiple of `align`, must not
	/// overflow `isize` (i.e. the rounded value must be less than or equal to
	/// `isize::MAX`)".
	pub const MAX_CAPACITY: usize = isize::MAX as usize - (Self::OUTPUT_ALIGNMENT - 1);

	/// Assertions for validity of alignment const params.
	/// These assertions are not evaluated here.
	/// `Self::ASSERT_ALIGNMENTS_VALID` must be referenced in all code paths
	/// creating a `BaseSerializer`, to ensure compile-time error if
	/// assertions fail.
	const ASSERT_ALIGNMENTS_VALID: () = {
		assert!(
			OUTPUT_ALIGNMENT < isize::MAX as usize,
			"OUTPUT_ALIGNMENT must be less than isize::MAX"
		);
		assert!(
			OUTPUT_ALIGNMENT.is_power_of_two(), // false if 0
			"OUTPUT_ALIGNMENT must be a power of 2"
		);
		assert!(
			MAX_VALUE_ALIGNMENT <= OUTPUT_ALIGNMENT,
			"MAX_VALUE_ALIGNMENT must be less than or equal to OUTPUT_ALIGNMENT",
		);
		assert!(
			MAX_VALUE_ALIGNMENT.is_power_of_two(), // false if 0
			"MAX_VALUE_ALIGNMENT must be a power of 2"
		);
		assert!(
			VALUE_ALIGNMENT <= MAX_VALUE_ALIGNMENT,
			"VALUE_ALIGNMENT must be less than or equal to MAX_VALUE_ALIGNMENT",
		);
		assert!(
			VALUE_ALIGNMENT.is_power_of_two(), // false if 0
			"VALUE_ALIGNMENT must be a power of 2"
		);
	};

	/// Create new Serializer from an existing `AlignedByteVec`
	/// or `&mut AlignedByteVec`.
	///
	/// # Panics
	///
	/// * If `buf` has 0 capacity
	/// * If `buf.capacity()` is not a multiple of `MAX_VALUE_ALIGNMENT`
	/// * If `buf.len()` is not a multiple of `VALUE_ALIGNMENT`
	pub fn from_vec(buf: Buf) -> Self {
		// Ensure (at compile time) that const params for alignment are valid
		let _ = Self::ASSERT_ALIGNMENTS_VALID;

		assert!(
			buf.borrow().capacity() > 0,
			"`buf.capacity()` must not be 0"
		);
		assert!(
			is_aligned_to(buf.borrow().capacity(), Self::MAX_VALUE_ALIGNMENT),
			"`buf.capacity()` must be a multiple of `MAX_VALUE_ALIGNMENT`"
		);
		assert!(
			is_aligned_to(buf.borrow().len(), Self::VALUE_ALIGNMENT),
			"`buf.len()` must be a multiple of `VALUE_ALIGNMENT`"
		);

		Self { buf }
	}

	/// Create new Serializer from an existing `AlignedByteVec`
	/// or `&mut AlignedByteVec` without checking invariants.
	///
	/// # Safety
	///
	/// * `buf` must have capacity greater than 0
	/// * `buf.capacity()` must be a multiple of `MAX_VALUE_ALIGNMENT`
	/// * `buf.len()` must be a multiple of `VALUE_ALIGNMENT`
	pub unsafe fn from_vec_unchecked(buf: Buf) -> Self {
		// Ensure (at compile time) that const params for alignment are valid
		let _ = Self::ASSERT_ALIGNMENTS_VALID;

		Self { buf }
	}

	/// Consume Serializer and return the output buffer as an `AlignedByteVec`
	/// or `&mut AlignedByteVec`.
	pub fn into_vec(self) -> Buf {
		self.buf
	}

	/// Align position in output buffer to alignment of `T`.
	#[inline]
	fn align_to<T>(&mut self) {
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
		if mem::align_of::<T>() > Self::VALUE_ALIGNMENT {
			// Static assertion above ensures `align()`'s constraints are satisfied
			unsafe { self.align(mem::align_of::<T>()) }
		}
	}

	/// Align position in output buffer to `alignment`.
	///
	/// # Safety
	///
	/// * `alignment` must be `<= MAX_VALUE_ALIGNMENT`
	/// * `alignment` must be a power of 2
	#[inline]
	unsafe fn align(&mut self, alignment: usize) {
		debug_assert!(alignment <= Self::MAX_VALUE_ALIGNMENT);
		debug_assert!(alignment.is_power_of_two());

		// Round up buffer position to multiple of `alignment`.
		// `align_up_to`'s constraints are satisfied by:
		// * `buf.len()` is always less than `MAX_CAPACITY`, which is `< isize::MAX`.
		// * `alignment <= MAX_VALUE_ALIGNMENT` satisfies `alignment < isize::MAX`
		//   because `MAX_VALUE_ALIGNMENT < isize::MAX`.
		// * `alignment` being a power of 2 is part of this function's contract.
		let new_pos = align_up_to(self.buf.borrow().len(), alignment);

		// `new_pos > capacity` can't happen because of 2 guarantees:
		// 1. `alignment <= MAX_VALUE_ALIGNMENT`
		// 2. `capacity` is a multiple of `MAX_VALUE_ALIGNMENT`
		self.buf.borrow_mut().set_len(new_pos);
	}

	/// Align position in output buffer to `VALUE_ALIGNMENT`.
	/// Does same as `align`, but a bit shorter as it can skip the check whether
	/// we can exceed capacity.
	#[inline]
	fn align_to_value_alignment(&mut self) {
		// `align_up_to`'s contract is easily fulfilled.
		// `buf.len()` is always `<= MAX_CAPACITY`.
		// `MAX_CAPACITY` and `VALUE_ALIGNMENT` are both `< isize::MAX`.
		let new_pos = align_up_to(self.buf.borrow().len(), Self::VALUE_ALIGNMENT);

		// Cannot result in `len > capacity` because we're only aligning to
		// `VALUE_ALIGNMENT` and `capacity` is always a multiple of this.
		unsafe { self.buf.borrow_mut().set_len(new_pos) };
	}
}

impl<
		Buf: BorrowMut<AlignedByteVec<OUTPUT_ALIGNMENT>>,
		const OUTPUT_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
	> Serializer for BaseSerializer<Buf, OUTPUT_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT>
{
	/// Push a slice of values into output buffer.
	#[inline]
	fn push_slice<T>(&mut self, slice: &[T]) {
		self.push_slice_raw(slice);
	}

	/// Push raw bytes to output buffer.
	#[inline]
	fn push_bytes(&mut self, bytes: &[u8]) {
		// Push bytes to buffer
		self.buf.borrow_mut().extend_from_slice(bytes);

		// Align buffer position to `VALUE_ALIGNMENT`, ready for the next value
		if Self::VALUE_ALIGNMENT > 1 {
			self.align_to_value_alignment();
		}
	}

	/// Push a slice of values into output buffer.
	#[inline]
	fn push_slice_raw<T>(&mut self, slice: &[T]) {
		// Align position in buffer to alignment of `T`
		self.align_to::<T>();

		// Calculating `size` can't overflow as that would imply this is a slice of
		// `usize::MAX + 1` or more bytes, which can't be possible.
		let size = mem::size_of::<T>() * slice.len();

		let buf = self.buf.borrow_mut();
		buf.reserve(size);

		unsafe {
			let src = slice.as_ptr();
			let dst = buf.as_mut_ptr().add(buf.len()) as *mut T;
			// `buf.reserve(size)` ensures there's enough allocated space in output buffer.
			// `src` must be correctly aligned as derived from a valid `&[T]`.
			// `dst` is aligned because of `self.align_to::<T>()` above.
			ptr::copy_nonoverlapping(src, dst, slice.len());
			buf.set_len(buf.len() + size);
		}

		// Align buffer position to `VALUE_ALIGNMENT`, ready for the next value.
		// This should be optimized away for types with alignment of `VALUE_ALIGNMENT`
		// or greater. Ditto for types which have lower alignment, but happen to have
		// size divisible by `VALUE_ALIGNMENT`. Hopefully this is the majority of types.
		// NB: Even though `size % Self::VALUE_ALIGNMENT` might produce a result of `0`
		// more often (e.g. if `VALUE_ALIGNMENT == 8`, `size_of::<T>() == 4` and
		// `slice.len() == 2`), just using `size_of::<T>()` here so the condition can be
		// statically evaluated and optimized out at compile time in most cases.
		if mem::size_of::<T>() % Self::VALUE_ALIGNMENT > 0 {
			self.align_to_value_alignment();
		}
	}

	/// Get current capacity of output.
	#[inline]
	fn capacity(&self) -> usize {
		self.buf.borrow().capacity()
	}

	/// Get current position in output.
	#[inline]
	fn pos(&self) -> usize {
		self.buf.borrow().len()
	}

	/// Move current position in output buffer.
	///
	/// # Safety
	///
	/// * `pos` must be less than or equal to `self.capacity()`.
	/// * `pos` must be a multiple of `VALUE_ALIGNMENT`.
	#[inline]
	unsafe fn set_pos(&mut self, pos: usize) {
		debug_assert!(pos <= self.buf.borrow().capacity());
		debug_assert!(is_aligned_to(pos, Self::VALUE_ALIGNMENT));

		self.buf.borrow_mut().set_len(pos);
	}
}

/// Round up `pos` to alignment of `alignment`.
///
/// `alignment` must be a power of 2.
///
/// Caller must ensure `pos + alignment` cannot overflow `usize`.
/// This is satisfied if both `pos` and `alignment` are less than `isize::MAX`.
///
/// Breaking these conditions will yield an incorrect result which could
/// cause UB later on due to mis-aligned data.
pub const fn align_up_to(pos: usize, alignment: usize) -> usize {
	debug_assert!(alignment.is_power_of_two());
	(pos + alignment - 1) & !(alignment - 1)
}

/// Check if `pos` is a multiple of `alignment`.
///
/// `alignment` must be a power of 2.
pub const fn is_aligned_to(pos: usize, alignment: usize) -> bool {
	debug_assert!(alignment.is_power_of_two());
	pos & (alignment - 1) == 0
}

/// Type for static assertion that types being serialized do not have a higher
/// alignment requirement than the alignment of the output buffer
struct AlignmentCheck<T, const MAX_VALUE_ALIGNMENT: usize> {
	_marker: PhantomData<T>,
}

impl<T, const MAX_VALUE_ALIGNMENT: usize> AlignmentCheck<T, MAX_VALUE_ALIGNMENT> {
	const ASSERT_ALIGNMENT_DOES_NOT_EXCEED: () = assert!(mem::align_of::<T>() <= MAX_VALUE_ALIGNMENT);
}
