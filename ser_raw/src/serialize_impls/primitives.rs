use std::num;

use crate::{Serialize, Serializer};

macro_rules! impl_primitive {
	($ty:ty) => {
		impl<S: Serializer> Serialize<S> for $ty {
			#[inline(always)]
			fn serialize_data(&self, _serializer: &mut S) {}
		}
	};
}

impl_primitive!(u8);
impl_primitive!(u16);
impl_primitive!(u32);
impl_primitive!(u64);
impl_primitive!(u128);
impl_primitive!(usize);

impl_primitive!(i8);
impl_primitive!(i16);
impl_primitive!(i32);
impl_primitive!(i64);
impl_primitive!(i128);
impl_primitive!(isize);

impl_primitive!(num::NonZeroU8);
impl_primitive!(num::NonZeroU16);
impl_primitive!(num::NonZeroU32);
impl_primitive!(num::NonZeroU64);
impl_primitive!(num::NonZeroU128);
impl_primitive!(num::NonZeroUsize);

impl_primitive!(num::NonZeroI8);
impl_primitive!(num::NonZeroI16);
impl_primitive!(num::NonZeroI32);
impl_primitive!(num::NonZeroI64);
impl_primitive!(num::NonZeroI128);
impl_primitive!(num::NonZeroIsize);

impl_primitive!(f32);
impl_primitive!(f64);

impl_primitive!(bool);
impl_primitive!(char);

impl_primitive!(());
