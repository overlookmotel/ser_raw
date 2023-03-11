use std::slice;

use crate::Serialize;

/// Serializers implement this trait.
pub trait Serializer: Sized {
	/// Serialize a value and all its dependencies.
	///
	/// The entry point for serializing, which user will call.
	fn serialize_value<T: Serialize<Self>>(&mut self, t: &T) {
		self.push_raw(t);
		t.serialize_data(self);
	}

	/// Push a value to output.
	///
	/// This is a value in a separate allocation, reached by a pointer
	/// (e.g. `Box<T>`), where `T` does not need further serialization.
	/// If `T` does need further serialization, use `push_and_process` instead.
	///
	/// Some Serializers may record/overwrite the pointer address.
	#[inline]
	fn push<T>(&mut self, value: &T) {
		self.push_slice(slice::from_ref(value));
	}

	/// Push a slice of values to output.
	///
	/// This is a slice in a separate allocation, reached by a pointer
	/// (e.g. `Vec<T>`), where `T` does not need further serialization.
	/// If `T` does need further serialization, use `push_and_process_slice`
	/// instead.
	///
	/// Some Serializers may record/overwrite the pointer address.
	#[inline]
	fn push_slice<T>(&mut self, slice: &[T]) {
		self.push_and_process_slice(slice, |_| {});
	}

	/// Push a value to output and continue processing the value.
	///
	/// This is a value in a separate allocation, reached by a pointer
	/// (e.g. `Box<T>`).
	///
	/// Some Serializers may record/overwrite the pointer address.
	#[inline]
	fn push_and_process<T, P: FnOnce(&mut Self)>(&mut self, t: &T, process: P) {
		self.push_and_process_slice(slice::from_ref(t), process);
	}

	/// Push a slice of values to output and continue processing content of the
	/// slice.
	///
	/// This is a slice in a separate allocation, reached by a pointer
	/// (e.g. `Vec<T>`).
	///
	/// Some Serializers may record/overwrite the pointer address.
	fn push_and_process_slice<T, P: FnOnce(&mut Self)>(&mut self, slice: &[T], process: P) -> ();

	/// Push raw bytes to output.
	///
	/// Unlike `push`, `push_slice`, `push_and_process` and
	/// `push_and_process_slice`, this is not for values for which a Serializer
	/// may need to record a pointer address.
	///
	/// Mainly for use in custom serialization functions, where output
	/// representation includes multiple parts, and Deserializer only
	/// needs to know the location of the first part.
	///
	/// ```
	/// struct MyStringProxy;
	/// impl SerializeWith<MyString, S: Serializer> for MyStringProxy {
	///   fn serialize_data_with(my_str: &MyString, serializer: &mut S) {
	///     // Serializer may record pointer to this
	///     serializer.push(&my_str.len());
	///     // No need to record pointer to this, as it's deductible from pointer to `len`
	///     serializer.push_bytes(my_str.as_slice());
	///   }
	/// }
	/// ```
	#[inline]
	fn push_bytes(&mut self, bytes: &[u8]) {
		self.push_raw_slice(bytes);
	}

	/// Push a value to output.
	///
	/// Unlike `push` and `push_and_process`, this is not for values for which a
	/// Serializer may need to record a pointer address.
	#[inline]
	fn push_raw<T>(&mut self, value: &T) {
		self.push_raw_slice(slice::from_ref(value));
	}

	/// Push a slice of values to output.
	///
	/// Unlike `push_slice` and `push_and_process_slice`, this is not for values
	/// for which a Serializer may need to record a pointer address.
	fn push_raw_slice<T>(&mut self, slice: &[T]) -> ();

	/// Get current capacity of output.
	fn capacity(&self) -> usize;

	/// Get current position in output.
	fn pos(&self) -> usize;
}
