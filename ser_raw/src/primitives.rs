use std::num;

use crate::{Serialize, Serializer};

impl<S: Serializer> Serialize<S> for u8 {}
impl<S: Serializer> Serialize<S> for u16 {}
impl<S: Serializer> Serialize<S> for u32 {}
impl<S: Serializer> Serialize<S> for u64 {}
impl<S: Serializer> Serialize<S> for u128 {}
impl<S: Serializer> Serialize<S> for usize {}

impl<S: Serializer> Serialize<S> for i8 {}
impl<S: Serializer> Serialize<S> for i16 {}
impl<S: Serializer> Serialize<S> for i32 {}
impl<S: Serializer> Serialize<S> for i64 {}
impl<S: Serializer> Serialize<S> for i128 {}
impl<S: Serializer> Serialize<S> for isize {}

impl<S: Serializer> Serialize<S> for num::NonZeroU8 {}
impl<S: Serializer> Serialize<S> for num::NonZeroU16 {}
impl<S: Serializer> Serialize<S> for num::NonZeroU32 {}
impl<S: Serializer> Serialize<S> for num::NonZeroU64 {}
impl<S: Serializer> Serialize<S> for num::NonZeroU128 {}
impl<S: Serializer> Serialize<S> for num::NonZeroUsize {}

impl<S: Serializer> Serialize<S> for num::NonZeroI8 {}
impl<S: Serializer> Serialize<S> for num::NonZeroI16 {}
impl<S: Serializer> Serialize<S> for num::NonZeroI32 {}
impl<S: Serializer> Serialize<S> for num::NonZeroI64 {}
impl<S: Serializer> Serialize<S> for num::NonZeroI128 {}
impl<S: Serializer> Serialize<S> for num::NonZeroIsize {}

impl<S: Serializer> Serialize<S> for f32 {}
impl<S: Serializer> Serialize<S> for f64 {}

impl<S: Serializer> Serialize<S> for bool {}
impl<S: Serializer> Serialize<S> for char {}

impl<S: Serializer> Serialize<S> for () {}
