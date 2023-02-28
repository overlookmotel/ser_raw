use crate::Serialize;

/// `ser_raw` Serializers implement this trait.
pub trait Serializer: Sized {
	/// Serialize a value.
	///
	/// The entry point for serializing, which user will call.
	fn serialize_value<T: Serialize>(&mut self, t: &T) -> ();

	/// Push a value to output.
	///
	/// This is a value in a separate allocation, reached by a pointer
	/// (e.g. `Box<T>`).
	/// Some Serializers may record/overwrite the pointer address.
	fn push<T: Serialize>(&mut self, t: &T) -> ();

	/// Push a slice of values to output.
	///
	/// This is a slice in a separate allocation, reached by a pointer
	/// (e.g. `Vec<T>`).
	/// Some Serializers may record/overwrite the pointer address.
	fn push_slice<T: Serialize>(&mut self, slice: &[T]) -> ();

	/// Push raw bytes to output.
	///
	/// Unlike `push` and `push_slice`, this is not for values for which a
	/// Serializer may need to record a pointer address.
	///
	/// Mainly for use in custom serialization functions, where output
	/// representation includes multiple parts, and Deserializer only
	/// needs to know the location of the first part.
	///
	/// ```
	/// struct MyStringProxy;
	/// impl SerializeWith<MyString> for MyStringProxy {
	///   fn serialize_data_with<S: Serializer>(my_str: &MyString, serializer: &mut S) {
	///     // Serializer may record pointer to this
	///     serializer.push(&my_str.len());
	///     // No need to record pointer to this, as it's deductible from pointer to `len`
	///     serializer.push_bytes(my_str.as_slice());
	///   }
	/// }
	/// ```
	fn push_bytes(&mut self, bytes: &[u8]) -> ();
}
