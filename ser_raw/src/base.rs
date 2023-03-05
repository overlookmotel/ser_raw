use std::{marker::PhantomData, mem, ptr};

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
pub struct BaseSerializer<const OUTPUT_ALIGNMENT: usize, const VALUE_ALIGNMENT: usize> {
	buf: AlignedByteVec<OUTPUT_ALIGNMENT>,
}

impl<const OUTPUT_ALIGNMENT: usize, const VALUE_ALIGNMENT: usize>
	BaseSerializer<OUTPUT_ALIGNMENT, VALUE_ALIGNMENT>
{
	/// Alignment of output buffer
	pub const OUTPUT_ALIGNMENT: usize = OUTPUT_ALIGNMENT;

	/// Typical alignment of values being serialized
	pub const VALUE_ALIGNMENT: usize = VALUE_ALIGNMENT;

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
			VALUE_ALIGNMENT <= OUTPUT_ALIGNMENT,
			"VALUE_ALIGNMENT must be less than or equal to OUTPUT_ALIGNMENT",
		);
		assert!(
			VALUE_ALIGNMENT.is_power_of_two(), // false if 0
			"VALUE_ALIGNMENT must be a power of 2"
		);
	};

	/// Create new Serializer with minimal memory pre-allocated.
	pub fn new() -> Self {
		// `VALUE_ALIGNMENT` trivially fulfills `with_capacity_unchecked`'s contract
		unsafe { Self::with_capacity_unchecked(Self::VALUE_ALIGNMENT) }
	}

	/// Create new Serializer with buffer pre-allocated with capacity of
	/// at least `capacity` bytes.
	///
	/// `capacity` will be rounded up to a multiple of `VALUE_ALIGNMENT`.
	/// `capacity` cannot be 0. Panics if so.
	///
	/// If you know, or can estimate, the amount of buffer space that's going to
	/// be needed in advance, allocating upfront with `with_capacity` can
	/// dramatically improve performance vs using `new`.
	pub fn with_capacity(capacity: usize) -> Self {
		// Bounds check `capacity`
		assert!(capacity != 0, "`capacity` must be at least VALUE_ALIGNMENT");
		assert!(
			capacity <= Self::MAX_CAPACITY,
			"`capacity` cannot exceed `isize::MAX + 1 - OUTPUT_ALIGNMENT`"
		);

		// Round up capacity to multiple of `VALUE_ALIGNMENT`.
		// Above assertions + static assertions for allowable values of
		// `VALUE_ALIGNMENT` satisify constraints of `align_up_to`.
		let capacity = align_up_to(capacity, Self::VALUE_ALIGNMENT);

		// Above ensures compliance with `with_capacity_unchecked`'s contract
		unsafe { Self::with_capacity_unchecked(capacity) }
	}

	/// Create new Serializer with buffer pre-allocated with capacity of
	/// exactly `capacity` bytes.
	///
	/// # Safety
	///
	/// * `capacity` cannot be 0.
	/// * `capacity` must be `<= MAX_CAPACITY`.
	/// * `capacity` must be a multiple of `VALUE_ALIGNMENT`.
	///
	/// Failure to obey these constraints may not produce UB immediately,
	/// but breaks assumptions the rest of the implementation relies on,
	/// so could cause arithmetic overflow or misaligned writes later on.
	pub unsafe fn with_capacity_unchecked(capacity: usize) -> Self {
		// Ensure (at compile time) that const params for alignment are valid
		let _ = Self::ASSERT_ALIGNMENTS_VALID;

		debug_assert!(capacity > 0);
		debug_assert!(capacity <= Self::MAX_CAPACITY);
		debug_assert!(is_aligned_to(capacity, Self::VALUE_ALIGNMENT));

		// TODO: Could be a little bit more efficient here.
		// `AlignedByteVec::with_capacity` repeats some of the checks already conducted.
		// But this function isn't called often, so probably not worth worrying about.
		Self {
			buf: AlignedByteVec::<OUTPUT_ALIGNMENT>::with_capacity(capacity),
		}
	}

	/// Consume Serializer and return the output buffer as an `AlignedByteVec`.
	pub fn into_vec(self) -> AlignedByteVec<OUTPUT_ALIGNMENT> {
		self.buf
	}

	/// Align position in output buffer to alignment of `T`.
	#[inline]
	fn align_to<T>(&mut self) {
		// Ensure (at compile time) that `T`'s alignment does not exceed alignment of
		// output buffer
		let _ = AlignmentCheck::<T, OUTPUT_ALIGNMENT>::ASSERT_ALIGNMENT_DOES_NOT_EXCEED;

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
	/// * `alignment` must be `<= OUTPUT_ALIGNMENT`
	/// * `alignment` must be a power of 2
	#[inline]
	unsafe fn align(&mut self, alignment: usize) {
		debug_assert!(alignment <= Self::OUTPUT_ALIGNMENT);
		debug_assert!(alignment.is_power_of_two());

		// Round up buffer position to multiple of `alignment`.
		// `align_up_to`'s constraints are satisfied by:
		// * `buf.len()` is always less than `MAX_CAPACITY`, which is `< isize::MAX`.
		// * `alignment <= OUTPUT_ALIGNMENT` satisfies `alignment < isize::MAX` because
		//   `OUTPUT_ALIGNMENT < isize::MAX`.
		// * `alignment` is a power of 2 is part of this function's contract.
		let new_pos = align_up_to(self.buf.len(), alignment);

		// Ensure `len > capacity` can't happen.
		// This check is unavoidable as we only guarantee that capacity is a multiple of
		// `VALUE_ALIGNMENT`, and alignment of serialized types can be higher.
		// So when aligning for a higher-alignment type, we can't assume there's already
		// sufficient capacity.
		// No point gating this with a static check for
		// `OUTPUT_ALIGNMENT > VALUE_ALIGNMENT` as this function is only called when
		// `alignment > VALUE_ALIGNMENT` anyway.
		// TODO: Actually could remove this with a 3rd const param `MAX_VALUE_ALIGN`
		// and constrain capacity to always be a multiple of `MAX_VALUE_ALIGN`.
		if self.buf.capacity() < new_pos {
			// This will grow buffer by at least enough
			self.reserve_for_alignment(alignment);
		}

		self.buf.set_len(new_pos);
	}

	/// Reserve space in output buffer to satisfy alignment.
	/// Not inlined into `align` to hint to compiler that taking this branch is
	/// uncommon.
	#[cold]
	fn reserve_for_alignment(&mut self, additional: usize) {
		// TODO: Could make this faster - `reserve()` contains an addition op
		// and a comparison which are not needed as we've done them already.
		// But `AlignedByteVec` has no public API for that.
		self.buf.reserve(additional);
	}

	/// Align position in output buffer to `VALUE_ALIGNMENT`.
	/// Does same as `align`, but a bit shorter as it can skip the check whether
	/// we can exceed capacity.
	#[inline]
	fn align_to_value_alignment(&mut self) {
		unsafe {
			// `align_up_to`'s contract is easily fulfilled.
			// `buf.len()` is always `<= MAX_CAPACITY`.
			// `MAX_CAPACITY` and `VALUE_ALIGNMENT` are both `< isize::MAX`.
			let new_pos = align_up_to(self.buf.len(), Self::VALUE_ALIGNMENT);
			// Cannot result in `len > capacity` because we're only aligning to
			// `VALUE_ALIGNMENT` and `capacity` is always a multiple of this.
			self.buf.set_len(new_pos);
		};
	}
}

impl<const O: usize, const V: usize> Serializer for BaseSerializer<O, V> {
	/// Push a slice of values into output buffer.
	#[inline]
	fn push_slice<T>(&mut self, slice: &[T]) {
		self.push_slice_raw(slice);
	}

	/// Push raw bytes to output buffer.
	#[inline]
	fn push_bytes(&mut self, bytes: &[u8]) {
		// Push bytes to buffer
		self.buf.extend_from_slice(bytes);

		// Align buffer position to `VALUE_ALIGNMENT`, ready for the next value
		self.align_to_value_alignment();
	}

	/// Push a slice of values into output buffer.
	#[inline]
	fn push_slice_raw<T>(&mut self, slice: &[T]) {
		// Align position in buffer to alignment of `T`
		// TODO: Combine this with reserving space for the slice itself.
		self.align_to::<T>();

		// Calculating `size` can't overflow as that would imply this is a slice of
		// `usize::MAX + 1` or more bytes, which can't be possible.
		let size = mem::size_of::<T>() * slice.len();
		self.buf.reserve(size);

		unsafe {
			let src = slice.as_ptr();
			let dst = self.buf.as_mut_ptr().add(self.buf.len()) as *mut T;
			// `buf.reserve(size)` ensures there's enough allocated space in output buffer.
			// `src` must be correctly aligned as derived from a valid `&[T]`.
			// `dst` is aligned because of `self.align_to::<T>()` above.
			ptr::copy_nonoverlapping(src, dst, slice.len());
			self.buf.set_len(self.buf.len() + size);
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
struct AlignmentCheck<T, const OUTPUT_ALIGNMENT: usize> {
	_marker: PhantomData<T>,
}

impl<T, const OUTPUT_ALIGNMENT: usize> AlignmentCheck<T, OUTPUT_ALIGNMENT> {
	const ASSERT_ALIGNMENT_DOES_NOT_EXCEED: () = assert!(mem::align_of::<T>() <= OUTPUT_ALIGNMENT);
}
