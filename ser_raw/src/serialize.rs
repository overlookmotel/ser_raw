use crate::Serializer;

/// Trait for types which can be serialized.
///
/// Usually implemented with the derive macro `#[derive(Serializer)]`.
///
/// # Example
///
/// ```
/// use ser_raw::{Serialize, Serializer};
///
/// struct Foo {
/// 	smalls: Vec<u8>,
/// 	bigs: Vec<u32>,
/// }
///
/// impl<S> Serialize<S> for Foo
/// where S: Serializer
/// {
/// 	fn serialize_data(&self, serializer: &mut S) {
/// 		self.smalls.serialize_data(serializer);
/// 		self.bigs.serialize_data(serializer);
/// 	}
/// }
/// ```
pub trait Serialize<Ser: Serializer> {
	#[allow(unused_variables)]
	#[inline(always)]
	fn serialize_data(&self, serializer: &mut Ser) {}
}

/// Trait for implementing an equivalent of `Serialize` on foreign types for
/// which it's not possible to implement `Serialize` directly due to orphan
/// rules. Use with `#[ser_with]`.
///
/// # Example
///
/// ```
/// use ser_raw::{Serialize, Serializer, SerializeWith, pos::Addr};
///
/// // The foreign type we want to be able to serialize
/// use num_bigint::BigUint;
///
/// // Our own type which contains the foreign type
/// #[derive(Serialize)]
/// struct Foo {
/// 	#[ser_with(BigUintProxy)]
/// 	big: BigUint,
/// }
///
/// struct BigUintProxy;
/// impl<S> SerializeWith<BigUint, S> for BigUintProxy
/// where S: Serializer
/// {
/// 	fn serialize_data_with(biguint: &BigUint, serializer: &mut S) {
/// 		let bytes = biguint.to_bytes_le();
/// 		let ptr_addr = S::Addr::from_ref(biguint);
/// 		serializer.push(&bytes.len(), ptr_addr);
/// 		serializer.push_raw_bytes(bytes.as_slice());
/// 	}
/// }
/// ```
pub trait SerializeWith<T, Ser: Serializer> {
	fn serialize_data_with(t: &T, serializer: &mut Ser) -> ();
}
