use std::{marker::PhantomData, mem};

use super::Storage;
use crate::util::aligned_max_capacity;

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
/// * 64-bit systems: `i64::MAX - 15`
/// * 32-bit systems: `i32::MAX - 15`
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

/// Type for static assertion that types being serialized do not have a higher
/// alignment requirement than the alignment of the output buffer
pub(crate) struct AlignmentCheck<T, const MAX_VALUE_ALIGNMENT: usize> {
	_marker: PhantomData<T>,
}

impl<T, const MAX_VALUE_ALIGNMENT: usize> AlignmentCheck<T, MAX_VALUE_ALIGNMENT> {
	pub const ASSERT_ALIGNMENT_DOES_NOT_EXCEED: () =
		assert!(mem::align_of::<T>() <= MAX_VALUE_ALIGNMENT);
}
