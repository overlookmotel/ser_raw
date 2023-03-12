/// Macro for creating Serializer impls.
///
/// Use to create macros that implement `Serializer` with default methods
/// overriden.
///
/// This is used internally to create e.g. `impl_pure_copy_serializer!`.
///
/// The following input variations can be used:
///
/// ```
/// impl_serializer!({}, MySer);
/// impl_serializer!({}, MySer<T>);
/// impl_serializer!({}, MySer<T, U, V>);
/// impl_serializer!({}, MySer<T, U> where T: Clone, U: Copy);
/// impl_serializer!({}, MySer<const N: u8>);
/// impl_serializer!({}, MySer<const N: u8, const O: u16>);
/// impl_serializer!({}, MySer<const N: u8; T, U>);
/// impl_serializer!({}, MySer<const N: u8; T, U> where T: Clone);
/// ```
///
/// # Syntax rules:
///
/// * Any const params must come first.
/// * If both const params and type params, the last const param must be
///   followed by a semicolon `;` *not* a `,`.
///
/// # Example
///
/// ```
/// macro_rules! impl_super_fast_serializer {
/// 	($($type_def:tt)*) => {
/// 		::ser_raw::impl_serializer!(
/// 			{
/// 				fn push_raw_slice<T>(&mut self, slice: &[T]) {
/// 					self.storage_mut().push_super_fast(slice);
/// 				}
/// 			},
/// 			$($type_def)*
/// 		);
/// 	};
/// }
/// ```
///
/// It can then be used to instantiate `SuperFastSerializer` with the augmented
/// `Serializer` methods defined in the macro.
///
/// ```
/// trait SuperFastSerializer {}
///
/// struct FastSer {
/// 	storage: FastStorage,
/// }
/// impl SuperFastSerializer for FastSer {}
/// impl_super_fast_serializer!(FastSer);
/// ```
#[macro_export]
macro_rules! impl_serializer {
	// `impl_serializer!({}, Foo)`
	({$($methods:item)*}, $ty:ident) => {
		impl $crate::Serializer for $ty {$($methods)*}
	};

	// `impl_serializer!({}, Foo<T>)`
	// `impl_serializer!({}, Foo<T, U, V>)`
	// `impl_serializer!({}, Foo<T> where T: Sized)`
	// `impl_serializer!({}, Foo<T, U> where T: Sized + Copy, U: Clone)`
	(
		{$($methods:item)*},
		$ty:ident<$first:ident $(,$more:ident)* $(,)?>
		$(where $($where:tt)+)?
	) => {
		impl<$first $(, $more)*> $crate::Serializer
		for $ty<$first $(, $more)*>
		$(where $($where)*)?
		{$($methods)*}
	};

	// `impl_serializer!({}, Foo<const N: u8>)`
	// `impl_serializer!({}, Foo<const N: u8, const O: u8>)`
	// `impl_serializer!({}, Foo<const N: u8> where N: IsValid<N>)`
	(
		{$($methods:item)*},
		$ty:ident<
			const $first_const:ident : $first_const_type:ty
			$(,const $more_const:ident : $more_const_type:ty)*
			$(,)?
		>
		$(where $($where:tt)+)?
	) => {
		impl<
			const $first_const: $first_const_type
			$(, const $more_const: $more_const_type)*
		> $crate::Serializer
		for $ty<$first_const $(, $more_const)*>
		$(where $($where)*)?
		{$($methods)*}
	};

	// `impl_serializer!({}, Foo<const N: u8; T, U, V>)`
	// `impl_serializer!({}, Foo<const N: u8, const O: u8; T, U, V>)`
	// `impl_serializer!({}, Foo<const N: u8; T> where T: Sized)`
	// `impl_serializer!({}, Foo<const N: u8, const O: u8; T, U> where T: Sized + Copy, U: Clone)`
	// NB: Const params must be first, followed by a `;` (not `,`).
	// Can't find a way to make `,` work as `const` after it is ambiguous
	// - could be a type called `const`.
	(
		{$($methods:item)*},
		$ty:ident<
			const $first_const:ident : $first_const_type:ty
			$(,const $more_const:ident : $more_const_type:ty)*;
			$first:ident $(,$more:ident)*
			$(,)?
		>
		$(where $($where:tt)+)?
	) => {
		impl<
			const $first_const: $first_const_type
			$(, const $more_const: $more_const_type)*,
			$first $(, $more)*
		> $crate::Serializer
		for $ty<$first_const $(, $more_const)*, $first $(, $more)*>
		$(where $($where)*)?
		{$($methods)*}
	};
}
