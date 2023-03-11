use std::borrow::BorrowMut;

use crate::{
	storage::{AlignedVec, Storage},
	Serializer,
};

/// Serializer that ensures values are correctly aligned in output buffer.
///
/// # Const parameters
///
/// `STORAGE_ALIGNMENT` is the alignment of the output buffer.
///
/// `MAX_VALUE_ALIGNMENT` is maximum alignment of types which will be
/// serialized. Types with alignment greater than `MAX_VALUE_ALIGNMENT` cannot
/// be serialized with this serializer.
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
	Store: BorrowMut<AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT>>,
	const STORAGE_ALIGNMENT: usize,
	const VALUE_ALIGNMENT: usize,
	const MAX_VALUE_ALIGNMENT: usize,
> {
	storage: Store,
}

impl<
		const STORAGE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
	>
	BaseSerializer<
		AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT>,
		STORAGE_ALIGNMENT,
		VALUE_ALIGNMENT,
		MAX_VALUE_ALIGNMENT,
	>
{
	/// Create new `BaseSerializer` with no memory pre-allocated.
	///
	/// If you know, or can estimate, the amount of buffer space that's going to
	/// be needed in advance, allocating upfront with `with_capacity` can
	/// dramatically improve performance vs using `new`.
	#[inline]
	pub fn new() -> Self {
		Self {
			storage: AlignedVec::new(),
		}
	}

	/// Create new `BaseSerializer` with buffer pre-allocated with capacity of
	/// at least `capacity` bytes.
	///
	/// `capacity` will be rounded up to a multiple of `MAX_VALUE_ALIGNMENT`.
	///
	/// # Panics
	///
	/// Panics if `capacity` exceeds `MAX_CAPACITY`.
	pub fn with_capacity(capacity: usize) -> Self {
		// `AlignedVec::with_capacity()` ensures capacity is `< MAX_CAPACITY`
		// and rounds up capacity to a multiple of `MAX_VALUE_ALIGNMENT`
		Self {
			storage: AlignedVec::with_capacity(capacity),
		}
	}
}

impl<
		Store: BorrowMut<AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT>>,
		const STORAGE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
	> BaseSerializer<Store, STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT>
{
	/// Alignment of output buffer
	pub const STORAGE_ALIGNMENT: usize = STORAGE_ALIGNMENT;

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
	pub const MAX_CAPACITY: usize = isize::MAX as usize - (STORAGE_ALIGNMENT - 1);

	/// Create new `BaseSerializer` from an existing `BorrowMut<AlignedVec>`.
	#[inline]
	pub fn from_storage(storage: Store) -> Self {
		// `AlignedVec` enforces the constraints we require:
		// * `capacity` does not exceed `MAX_CAPACITY`
		// * `capacity` is a multiple of `MAX_VALUE_ALIGNMENT`
		// * `len` is a multiple of `VALUE_ALIGNMENT`
		Self { storage }
	}

	/// Consume Serializer and return the output as a `BorrowMut<AlignedVec>`.
	#[inline]
	pub fn into_storage(self) -> Store {
		self.storage
	}
}

impl<
		Store: BorrowMut<AlignedVec<STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT>>,
		const STORAGE_ALIGNMENT: usize,
		const VALUE_ALIGNMENT: usize,
		const MAX_VALUE_ALIGNMENT: usize,
	> Serializer for BaseSerializer<Store, STORAGE_ALIGNMENT, VALUE_ALIGNMENT, MAX_VALUE_ALIGNMENT>
{
	/// Push a slice of values to output and continue processing content of the
	/// slice.
	#[inline]
	fn push_and_process_slice<T, P: FnOnce(&mut Self)>(&mut self, slice: &[T], process: P) {
		self.push_raw_slice(slice);
		process(self);
	}

	/// Push a slice of values into output buffer.
	#[inline]
	fn push_raw_slice<T>(&mut self, slice: &[T]) {
		self.storage.borrow_mut().push_slice(slice);
	}

	/// Get current capacity of output.
	#[inline]
	fn capacity(&self) -> usize {
		self.storage.borrow().capacity()
	}

	/// Get current position in output.
	#[inline]
	fn pos(&self) -> usize {
		self.storage.borrow().len()
	}
}
