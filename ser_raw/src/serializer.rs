use std::{borrow::BorrowMut, slice};

use crate::{pos::Addr, storage::Storage, Serialize};

/// Serializers implement this trait.
///
/// # With derive macro
///
///	The simplest way to build a custom [`Serializer`] is with the derive macro.
///
/// * `#[derive(Serializer)]`
/// * `#[ser_type(...)]` with the type of serializer you're building.
/// * Add required fields and tag them e.g. `#[ser_storage]` (see examples
///   below).
///
/// ## Pure copy serializer
///
/// [`PureCopySerializer`]-style serializer:
///
/// ```
/// use ser_raw::{
/// 	storage::{AlignedVec, Storage},
/// 	util::aligned_max_capacity,
/// 	Serializer,
/// };
///
/// const MAX_CAPACITY: usize = aligned_max_capacity(16);
/// type Store = AlignedVec<16, 16, 8, MAX_CAPACITY>;
///
/// #[derive(Serializer)]
/// #[ser_type(pure_copy)]
/// struct MySer {
/// 	#[ser_storage(Store)]
/// 	storage: Store,
/// }
///
/// impl MySer {
/// 	pub fn new() -> MySer {
/// 		MySer {
/// 			storage: Store::new(),
/// 		}
/// 	}
/// }
/// ```
///
/// ## Pointer offset serializer
///
/// [`PtrOffsetSerializer`]-style:
///
/// ```
/// use ser_raw::{
/// 	pos::PosMapping,
/// 	storage::{AlignedVec, Storage},
/// 	util::aligned_max_capacity,
/// 	Serializer,
/// };
///
/// const MAX_CAPACITY: usize = aligned_max_capacity(16);
/// type Store = AlignedVec<16, 16, 8, MAX_CAPACITY>;
///
/// #[derive(Serializer)]
/// #[ser_type(ptr_offset)]
/// struct MySer {
/// 	#[ser_storage(Store)]
/// 	storage: Store,
/// 	#[ser_pos_mapping]
/// 	pos_mapping: PosMapping,
/// }
///
/// impl MySer {
/// 	pub fn new() -> MySer {
/// 		MySer {
/// 			storage: Store::new(),
/// 			pos_mapping: PosMapping::dummy(),
/// 		}
/// 	}
/// }
/// ```
///
/// ## Complete serializer
///
/// [`CompleteSerializer`]-style:
///
/// ```
/// use ser_raw::{
/// 	pos::{PosMapping, Ptrs},
/// 	storage::{AlignedVec, Storage},
/// 	util::aligned_max_capacity,
/// 	Serializer,
/// };
///
/// const MAX_CAPACITY: usize = aligned_max_capacity(16);
/// type Store = AlignedVec<16, 16, 8, MAX_CAPACITY>;
///
/// #[derive(Serializer)]
/// #[ser_type(complete)]
/// struct MySer {
/// 	#[ser_storage(Store)]
/// 	storage: Store,
/// 	#[ser_pos_mapping]
/// 	pos_mapping: PosMapping,
/// 	#[ser_ptrs]
/// 	ptrs: Ptrs,
/// }
///
/// impl MySer {
/// 	pub fn new() -> MySer {
/// 		MySer {
/// 			storage: Store::new(),
/// 			pos_mapping: PosMapping::dummy(),
/// 			ptrs: Ptrs::new(),
/// 		}
/// 	}
/// }
/// ```
///
/// # Manual implementation
///
/// Implementers only need to implement the methods to access storage:
///
/// * [`storage`](Serializer::storage)
/// * [`storage_mut`](Serializer::storage_mut)
/// * [`into_storage`](Serializer::into_storage)
///
/// and the associated types:
///
/// * [`Storage`](Serializer::Storage)
/// * [`BorrowedStorage`](Serializer::BorrowedStorage)
/// * [`Addr`](Serializer::Addr)
///
/// Default implementation of all other methods delegates calls to the
/// underlying [`Storage`]. This produces the behavior of a "pure copy"
/// serializer (e.g. [`PureCopySerializer`]).
///
/// Other methods can be overriden to produce more complicated behavior, as is
/// the case with other serializers this crate provides e.g.
/// [`CompleteSerializer`].
///
/// # Example
///
/// This is a slightly simplified version of [`PureCopySerializer`]:
///
/// ```
/// use ser_raw::{
/// 	pos::NoopAddr,
/// 	storage::{AlignedVec, Storage},
/// 	util::aligned_max_capacity,
/// 	Serializer,
/// };
///
/// const MAX_CAPACITY: usize = aligned_max_capacity(16);
/// type Store = AlignedVec<16, 16, 8, MAX_CAPACITY>;
///
/// struct MySerializer {
/// 	storage: Store,
/// }
///
/// impl MySerializer {
/// 	fn new() -> Self {
/// 		Self { storage: Store::new() }
/// 	}
/// }
///
/// impl Serializer for MySerializer {
/// 	type Storage = Store;
/// 	type BorrowedStorage = Store;
/// 	type Addr = NoopAddr;
///
/// 	fn storage(&self) -> &Store { &self.storage }
/// 	fn storage_mut(&mut self) -> &mut Store { &mut self.storage }
/// 	fn into_storage(self) -> Store { self.storage }
/// }
/// ```
///
/// [`PureCopySerializer`]: crate::PureCopySerializer
/// [`PtrOffsetSerializer`]: crate::PtrOffsetSerializer
/// [`CompleteSerializer`]: crate::CompleteSerializer
pub trait Serializer: Sized {
	/// [`Storage`] which backs this serializer.
	type Storage: Storage;

	/// `BorrowMut` of [`Storage`] which backs this serializer.
	/// This enables creating a serializer from an existing [`Storage`].
	type BorrowedStorage: BorrowMut<Self::Storage>;

	/// [`Addr`] type this serializer uses.
	type Addr: Addr;

	/// Serialize a value and all its dependencies.
	///
	/// This is the entry point for serializing, when serializing a single value.
	///
	/// Consume serializer and return backing storage as `BorrowMut<Storage>`,
	/// along with position of the serialized value in storage.
	///
	/// # Example
	///
	/// ```
	/// use ser_raw::{
	/// 	storage::Storage,
	/// 	util::aligned_max_capacity,
	/// 	PureCopySerializer, Serialize, Serializer,
	/// };
	///
	/// #[derive(Serialize)]
	/// struct Foo {
	/// 	small: u8,
	/// 	big: u32,
	/// }
	///
	/// const MAX_CAPACITY: usize = aligned_max_capacity(16);
	/// let mut ser = PureCopySerializer::<16, 16, 8, MAX_CAPACITY, _>::new();
	/// let (pos, storage) = ser.serialize(&Foo { small: 1, big: 2 });
	/// assert_eq!(pos, 0);
	/// assert_eq!(storage.len(), 8);
	/// ```
	fn serialize<T: Serialize<Self>>(mut self, value: &T) -> (usize, Self::BorrowedStorage) {
		let pos = self.serialize_value(value);
		let storage = self.finalize();
		(pos, storage)
	}

	/// Serialize a value and all its dependencies.
	///
	/// This is the entry point for serializing, when serializing multiple values
	/// with a single [`Serializer`].
	///
	/// Call `serialize_value` multiple times, and then [`finalize`] to get
	/// output.
	///
	/// Returns position of value in output.
	///
	/// # Example
	///
	/// ```
	/// use ser_raw::{
	/// 	storage::Storage,
	/// 	util::aligned_max_capacity,
	/// 	PureCopySerializer, Serialize, Serializer,
	/// };
	///
	/// #[derive(Serialize)]
	/// struct Foo {
	/// 	small: u8,
	/// 	big: u32,
	/// }
	///
	/// const MAX_CAPACITY: usize = aligned_max_capacity(16);
	/// let mut ser = PureCopySerializer::<16, 16, 8, MAX_CAPACITY, _>::new();
	/// let pos1 = ser.serialize_value(&Foo { small: 1, big: 2 });
	/// let pos2 = ser.serialize_value(&Foo { small: 3, big: 4 });
	/// let storage = ser.finalize();
	///
	/// assert_eq!(storage.len(), 16);
	/// assert_eq!(pos1, 0);
	/// assert_eq!(pos2, 8);
	/// ```
	///
	/// [`finalize`]: Serializer::finalize
	fn serialize_value<T: Serialize<Self>>(&mut self, value: &T) -> usize {
		// Align storage, ready to write value, and get position
		self.storage_mut().align_for::<T>();
		let pos = self.pos();

		// Push value to storage.
		// `push_slice_unaligned`'s requirements are satisfied by `align_for::<T>()` and
		// `align_after::<T>()`.
		let slice = slice::from_ref(value);
		unsafe { self.storage_mut().push_slice_unaligned(slice) };
		self.storage_mut().align_after::<T>();

		// Serialize value
		value.serialize_data(self);

		// Return position value was written at
		pos
	}

	/// Push a value to output.
	///
	/// This is a value in a separate allocation, reached by a pointer
	/// (e.g. `Box<T>`), where `T` does not need further serialization.
	/// If `T` does need further serialization, use
	/// [`push_and_process`](Serializer::push_and_process) instead.
	///
	/// Some Serializers may record/overwrite the pointer address.
	#[inline]
	fn push<T>(&mut self, value: &T, ptr_addr: Self::Addr) {
		self.push_slice(slice::from_ref(value), ptr_addr);
	}

	/// Push a slice of values to output.
	///
	/// This is a slice in a separate allocation, reached by a pointer
	/// (e.g. `Vec<T>`), where `T` does not need further serialization.
	/// If `T` does need further serialization, use
	/// [`push_and_process_slice`](Serializer::push_and_process_slice) instead.
	///
	/// Some Serializers may record/overwrite the pointer address.
	#[inline]
	fn push_slice<T>(&mut self, slice: &[T], ptr_addr: Self::Addr) {
		self.push_and_process_slice(slice, ptr_addr, |_| {});
	}

	/// Push a value to output and continue processing the value.
	///
	/// The value will be added to output, and then `process()` called.
	///
	/// This is a value in a separate allocation, reached by a pointer
	/// (e.g. `Box<T>`).
	///
	/// Some Serializers may record/overwrite the pointer address.
	#[inline]
	fn push_and_process<T, P: FnOnce(&mut Self)>(&mut self, t: &T, ptr_addr: Self::Addr, process: P) {
		self.push_and_process_slice(slice::from_ref(t), ptr_addr, process);
	}

	/// Push a slice of values to output and continue processing content of the
	/// slice.
	///
	/// The value will be added to output, and then `process()` called.
	///
	/// This is a slice in a separate allocation, reached by a pointer
	/// (e.g. `Vec<T>`).
	///
	/// Some Serializers may record/overwrite the pointer address.
	#[inline]
	fn push_and_process_slice<T, P: FnOnce(&mut Self)>(
		&mut self,
		slice: &[T],
		#[allow(unused_variables)] ptr_addr: Self::Addr,
		process: P,
	) {
		self.push_raw_slice(slice);
		process(self);
	}

	/// Push a value to output.
	///
	/// Unlike [`push`](Serializer::push) and
	/// [`push_and_process`](Serializer::push_and_process), this is not for values
	/// for which a Serializer may need to record a pointer address.
	#[inline]
	fn push_raw<T>(&mut self, value: &T) {
		self.push_raw_slice(slice::from_ref(value));
	}

	/// Push a slice of values to output.
	///
	/// Unlike [`push_slice`](Serializer::push_slice) and
	/// [`push_and_process_slice`](Serializer::push_and_process_slice), this is
	/// not for values for which a Serializer may need to record a pointer
	/// address.
	#[inline]
	fn push_raw_slice<T>(&mut self, slice: &[T]) {
		self.storage_mut().push_slice(slice);
	}

	/// Push raw bytes to output.
	///
	/// Unlike [`push`](Serializer::push), [`push_slice`](Serializer::push_slice),
	/// [`push_and_process`](Serializer::push_and_process) and
	/// [`push_and_process_slice`](Serializer::push_and_process_slice), this is
	/// not for values for which a Serializer may need to record a pointer
	/// address.
	///
	/// Mainly for use in custom serialization functions, where output
	/// representation includes multiple parts, and Deserializer only
	/// needs to know the location of the first part.
	///
	/// # Example
	///
	/// ```
	/// use ser_raw::{Serializer, SerializeWith, pos::Addr};
	///
	/// struct MyString { inner: String }
	///
	/// struct MyStringProxy;
	/// impl<S> SerializeWith<MyString, S> for MyStringProxy
	/// where S: Serializer
	/// {
	/// 	fn serialize_data_with(my_str: &MyString, serializer: &mut S) {
	/// 		// Serializer may record pointer to this.
	/// 		// Actually next line is not quite right -
	/// 		// We need address of the pointer inside `String`, not `String` itself.
	/// 		let ptr_addr = S::Addr::from_ref(&my_str.inner);
	/// 		serializer.push(&my_str.inner.len(), ptr_addr);
	/// 		// No need to record pointer to this, as it's deductible from pointer to `len`
	/// 		serializer.push_raw_bytes(my_str.inner.as_bytes());
	/// 	}
	/// }
	/// ```
	#[inline]
	fn push_raw_bytes(&mut self, bytes: &[u8]) {
		self.storage_mut().push_bytes(bytes);
	}

	/// Advance storage position to leave space to write a `T` at current position
	/// later.
	///
	/// Will also insert padding as required, to ensure the `T` can be written
	/// with correct alignment.
	#[inline]
	fn push_empty<T>(&mut self) {
		self.storage_mut().push_empty::<T>();
	}

	/// Advance storage position to leave space to write a slice `&[T]` at current
	/// position later.
	///
	/// Will also insert padding as required, to ensure the `&[T]` can be written
	/// with correct alignment.
	///
	/// If the size of the slice is known statically, prefer
	/// `push_empty::<[T; N]>()` to `push_empty_slice::<T>(N)`,
	/// as the former is slightly more efficient.
	#[inline]
	fn push_empty_slice<T>(&mut self, len: usize) {
		self.storage_mut().push_empty_slice::<T>(len);
	}

	/// Write a value to storage at a specific position.
	///
	/// Default implementation is a no-op, and some serializers may not need to
	/// implement a functional version of this, if they don't need/support writing
	/// to storage at arbitrary positions.
	///
	/// A Serializer can also opt to implement this method not by writing to the
	/// storage immediately, but instead storing the details of what need to be
	/// written in some other form, and to leaving it to the deserializer to
	/// perform the writes later.
	///
	/// # Safety
	///
	/// `pos + mem::size_of::<T>()` must be less than or equal to `capacity()`.
	/// i.e. `pos` must be within bounds of the currently allocated storage.
	#[allow(unused_variables)]
	#[inline(always)]
	unsafe fn write<T>(&mut self, addr: usize, value: &T) {
		// TODO: Would be better to take an `Addr`
		// TODO: The doc comment above is wrong. `addr` is an address of an original
		// value, not position in the output.
	}

	/// Write a correction to storage.
	///
	/// An example of a "correction" is: Serializing a `Vec` which has
	/// `capacity` of 2, but `len` of 1. The correction is amending the `capacity`
	/// field to 1, to reflect that the copy of the `Vec` in serialized output
	/// is shrunk to fit, and only contains 1 element, with no additional
	/// capacity.
	///
	/// Default implementation is a no-op, and some serializers may not need to
	/// implement a functional version of this, if they don't need corrections.
	///
	/// Method takes a closure, so that [`Serialize::serialize_data`]
	/// implementations can perform operations which may have some cost in the
	/// closure, prior to performing writes. If the [`Serializer`] doesn't care
	/// about corrections and uses this default no-op implementation of
	/// `write_correction`, the closure will not be called and the cost of those
	/// operations is avoided. Hopefully the compiler will recognise this and
	/// remove the call to `write_correction` and the code inside the closure
	/// entirely, so it's completely zero cost unless it's used.
	///
	/// If Serializer *does* want to receive corrections, it would implement this
	/// method as:
	/// ```ignore
	/// fn write_correction<W: FnOnce(&mut Self)>(&mut self, write: W) {
	/// 	write(self);
	/// }
	/// ```
	#[allow(unused_variables)]
	#[inline(always)]
	fn write_correction<W: FnOnce(&mut Self)>(&mut self, write: W) {}

	/// Finalize serialization, consume serializer, and return backing storage as
	/// `BorrowMut<Storage>`.
	#[inline]
	fn finalize(self) -> Self::BorrowedStorage {
		self.into_storage()
	}

	/// Get current capacity of output.
	#[inline]
	fn capacity(&self) -> usize {
		self.storage().capacity()
	}

	/// Get current position in output.
	#[inline]
	fn pos(&self) -> usize {
		self.storage().len()
	}

	/// Get immutable ref to [`Storage`] backing this [`Serializer`].
	fn storage(&self) -> &Self::Storage;

	/// Get mutable ref to [`Storage`] backing this [`Serializer`].
	fn storage_mut(&mut self) -> &mut Self::Storage;

	/// Consume Serializer and return the backing storage as a
	/// `BorrowMut<Storage>`.
	///
	/// Consumers should not call this method directly. Call
	/// [`finalize`](Serializer::finalize) instead, as some serializers need to
	/// make final changes to the output at the end of serialization.
	fn into_storage(self) -> Self::BorrowedStorage;
}
