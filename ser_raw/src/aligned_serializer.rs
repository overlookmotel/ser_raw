use std::{mem, slice};

use crate::{AlignedByteVec, Serialize, Serializer};

/// Serializer that ensures objects are correctly aligned in output buffer.
///
/// `OUTPUT_ALIGNMENT` is the alignment of the output buffer.
/// Types with alignment greater than `OUTPUT_ALIGNMENT` cannot be serialized
/// with this serializer.
///
/// `VALUE_ALIGNMENT` is minimum alignment all values will have in output
/// buffer. This doesn't affect the "legality" of the output, but if most types
/// being serialized have same alignment, setting `VALUE_ALIGNMENT`
/// to that alignment may improve performance, as alignment arithmetic
/// calculations can be skipped in most cases.
///
/// e.g. If all (or almost all) types contain pointers (`Box`, `Vec` etc),
/// setting `VALUE_ALIGNMENT = std::mem::size_of::<usize>()`
/// will be the best value for fast serialization.
///
/// The higher `VALUE_ALIGNMENT` is, the more padding bytes will end up on
/// output, potentially increasing output size significantly, depending on the
/// types being serialized.
pub struct AlignedSerializer<const OUTPUT_ALIGNMENT: usize, const VALUE_ALIGNMENT: usize> {
	buf: AlignedByteVec<OUTPUT_ALIGNMENT>,
}

impl<const OUTPUT_ALIGNMENT: usize, const VALUE_ALIGNMENT: usize>
	AlignedSerializer<OUTPUT_ALIGNMENT, VALUE_ALIGNMENT>
{
	const ASSERT_ALIGNMENTS_VALID: () = {
		assert!(OUTPUT_ALIGNMENT > 0, "OUTPUT_ALIGNMENT must be 1 or more");
		assert!(
			OUTPUT_ALIGNMENT < isize::MAX as usize,
			"OUTPUT_ALIGNMENT must be less than isize::MAX"
		);
		assert!(
			OUTPUT_ALIGNMENT == OUTPUT_ALIGNMENT.next_power_of_two(),
			"OUTPUT_ALIGNMENT must be a power of 2"
		);
		assert!(VALUE_ALIGNMENT > 0, "VALUE_ALIGNMENT must be 1 or more");
		assert!(
			VALUE_ALIGNMENT <= OUTPUT_ALIGNMENT,
			"VALUE_ALIGNMENT must be less than or equal to OUTPUT_ALIGNMENT",
		);
		assert!(
			VALUE_ALIGNMENT == VALUE_ALIGNMENT.next_power_of_two(),
			"VALUE_ALIGNMENT must be a power of 2"
		);
	};
	/// Maximum capacity of output buffer.
	/// Dictated by the requirements of
	/// [`alloc::Layout`](https://doc.rust-lang.org/alloc/alloc/struct.Layout.html).
	/// "`size`, when rounded up to the nearest multiple of `align`, must not
	/// overflow `isize` (i.e. the rounded value must be less than or equal to
	/// `isize::MAX`)".
	pub const MAX_CAPACITY: usize = isize::MAX as usize - (Self::OUTPUT_ALIGNMENT - 1);
	pub const OUTPUT_ALIGNMENT: usize = OUTPUT_ALIGNMENT;
	pub const VALUE_ALIGNMENT: usize = VALUE_ALIGNMENT;

	/// Create new Serializer with minimal memory pre-allocated.
	/// without allocating any memory for output buffer.
	/// Memory will be allocated when first object is serialized.
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
	/// Safety:
	///
	/// Caller must ensure:
	/// * `capacity` is not 0.
	/// * `capacity <= MAX_CAPACITY`.
	/// * `capacity` is a multiple of `VALUE_ALIGNMENT`.
	///
	/// Failure to obey these constraints may not produce UB immediately,
	/// but breaks assumptions other code here relies on, so could cause
	/// arthmetic overflow or alignment problems later on.
	unsafe fn with_capacity_unchecked(capacity: usize) -> Self {
		let _ = Self::ASSERT_ALIGNMENTS_VALID;

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
	/// This should be optimized away for types with alignment of
	/// `VALUE_ALIGNMENT` or less. Hopefully this is the majority of types.
	#[inline]
	fn align_to<T>(&mut self) {
		if mem::align_of::<T>() > Self::VALUE_ALIGNMENT {
			// Constraints of `align()` are satisfied if
			// `align_of::<T>() <= OUTPUT_ALIGNMENT`.
			// TODO: This isn't currently guaranteed, but it should be.
			unsafe { self.align(mem::align_of::<T>()) }
		}
	}

	/// Align position in output buffer to `alignment`.
	///
	/// Safety:
	///
	/// Caller must ensure:
	/// * `alignment <= OUTPUT_ALIGNMENT`
	/// * `alignment` is a power of 2
	#[inline]
	unsafe fn align(&mut self, alignment: usize) {
		// Round up buffer position to multiple of `alignment`.
		// `align_up_to`'s constraints are satisfied by:
		// * `buf.len()` is always less than `MAX_CAPACITY`, which is `< isize::MAX`.
		// * `alignment <= OUTPUT_ALIGNMENT` satisfies `alignment < isize::MAX` because
		//   `OUTPUT_ALIGNMENT < isize::MAX`.
		// * `alignment` is a power of 2 is part of this function's contract.
		let new_pos = align_up_to(self.buf.len(), alignment);

		// Ensure `len > capacity` can't happen.
		// This check is unavoidable as we only guarantee that capacity is a multiple of
		// `VALUE_ALIGNMENT`, and `OUTPUT_ALIGNMENT` can be higher.
		// No point gating this with a static check for
		// `OUTPUT_ALIGNMENT > VALUE_ALIGNMENT` as this function is only called when
		// `alignment > VALUE_ALIGNMENT` anyway.
		// TODO: Actually could remove this with a 3rd const param `MAX_VALUE_ALIGN`
		// and constrain capacity so it's always a multiple of that.
		if self.buf.capacity() < new_pos {
			// This will grow buffer by at least enough.
			// Separate function to hint to compiler that taking the branch is uncommon.
			// TODO: Could make this faster - `reserve()` contains an addition op
			// and a comparison which are not needed as we've done them already.
			// But `AlignedByteVec` has no public API for that.
			self.reserve_for_alignment(alignment);
		}

		self.buf.set_len(new_pos);
	}

	/// Reserve space in output buffer.
	#[inline(never)]
	fn reserve_for_alignment(&mut self, additional: usize) {
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

impl<const O: usize, const V: usize> Serializer for AlignedSerializer<O, V> {
	fn serialize_value<T: Serialize>(&mut self, t: &T) {
		self.push(t);
		t.serialize_data(self);
	}

	#[inline]
	fn push<T: Serialize>(&mut self, t: &T) {
		// TODO: Add const assertion that `align_of::<T>() <= `OUTPUT_ALIGNMENT`.
		// Not sure how to make it a const assert.
		// Or maybe a non-const `assert!()` would be optimized away anyway?

		// Align position in buffer to alignment of `T`
		self.align_to::<T>();

		// Write object to output
		// TODO: Use typed copy instead
		let ptr = t as *const T as *const u8;
		let bytes = unsafe { slice::from_raw_parts(ptr, mem::size_of::<T>()) };
		self.buf.extend_from_slice(bytes);

		// Align buffer position to `VALUE_ALIGNMENT`, ready for the next object.
		// This should be optimized away for types with alignment of `VALUE_ALIGNMENT`
		// or greater. Ditto for types which have lower alignment, but happen to have
		// size divisible by `VALUE_ALIGNMENT`. Hopefully this is the majority of types.
		if mem::size_of::<T>() % Self::VALUE_ALIGNMENT > 0 {
			self.align_to_value_alignment();
		}
	}

	#[inline]
	fn push_slice<T: Serialize>(&mut self, slice: &[T]) {
		// TODO: Add const assertion that `align_of::<T>() <= `OUTPUT_ALIGNMENT`.
		// Not sure how to make it a const assert.
		// Or maybe a non-const `assert!()` would be optimized away anyway?

		// Align position in buffer to alignment of `T`
		self.align_to::<T>();

		// Write slice to output
		// TODO: Use typed copy instead
		let ptr = slice.as_ptr() as *const u8;
		let bytes = unsafe { slice::from_raw_parts(ptr, slice.len() * mem::size_of::<T>()) };
		self.push_bytes(bytes);

		// Align buffer position to `VALUE_ALIGNMENT`, ready for the next object.
		// This should be optimized away for types with alignment of `VALUE_ALIGNMENT`
		// or greater. Ditto for types which have lower alignment, but happen to have
		// size divisible by `VALUE_ALIGNMENT`. Hopefully this is the majority of types.
		if mem::size_of::<T>() % Self::VALUE_ALIGNMENT > 0 {
			self.align_to_value_alignment();
		}
	}

	#[inline]
	fn push_bytes(&mut self, bytes: &[u8]) {
		// Push bytes to buffer
		self.buf.extend_from_slice(bytes);

		// Align buffer position to `VALUE_ALIGNMENT`, ready for the next object
		self.align_to_value_alignment();
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
const fn align_up_to(pos: usize, alignment: usize) -> usize {
	(pos + alignment - 1) & !(alignment - 1)
}
