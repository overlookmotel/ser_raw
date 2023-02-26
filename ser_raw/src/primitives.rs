use std::num;

use crate::Serialize;

impl Serialize for u8 {}
impl Serialize for u16 {}
impl Serialize for u32 {}
impl Serialize for u64 {}
impl Serialize for u128 {}
impl Serialize for usize {}

impl Serialize for i8 {}
impl Serialize for i16 {}
impl Serialize for i32 {}
impl Serialize for i64 {}
impl Serialize for i128 {}
impl Serialize for isize {}

impl Serialize for num::NonZeroU8 {}
impl Serialize for num::NonZeroU16 {}
impl Serialize for num::NonZeroU32 {}
impl Serialize for num::NonZeroU64 {}
impl Serialize for num::NonZeroU128 {}
impl Serialize for num::NonZeroUsize {}

impl Serialize for num::NonZeroI8 {}
impl Serialize for num::NonZeroI16 {}
impl Serialize for num::NonZeroI32 {}
impl Serialize for num::NonZeroI64 {}
impl Serialize for num::NonZeroI128 {}
impl Serialize for num::NonZeroIsize {}

impl Serialize for f32 {}
impl Serialize for f64 {}

impl Serialize for bool {}
impl Serialize for char {}

impl Serialize for () {}
