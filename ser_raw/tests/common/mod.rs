mod minecraft_data;
pub use minecraft_data::generate_data as generate_minecraft_data;

pub enum Test {
	Primitives,
	NonZeroNumbers,
	Arrays,
	ArraysOfBoxes,
	Tuples,
	EnumFieldless,
	EnumWithFields,
	BoxedPrimitives,
	BoxedStructs,
	VecOfPrimitives,
	VecOfVecs,
	VecsWithZeroLenZeroCapacity,
	VecsWithZeroLenExcessCapacity,
	VecsWithZeroLenExcessCapacity2,
	VecsWithExcessCapacity,
	VecsWithExcessCapacity2,
	Strings,
	StringsWithZeroLenZeroCapacity,
	StringsWithZeroLenExcessCapacity,
	StringsWithZeroLenExcessCapacity2,
	StringsWithExcessCapacity,
	StringsWithExcessCapacity2,
	Options,
	StructureWhereStorageGrowsAfterLastPointerWritten,
	MinecraftData,
}

macro_rules! tests {
	($test_serialize:ident) => {
		#[test]
		fn primitives() {
			#[derive(Serialize, Debug, PartialEq)]
			struct Foo {
				u8: u8,
				u16: u16,
				u32: u32,
				u64: u64,
				u128: u128,
				i8: i8,
				i16: i16,
				i32: i32,
				i64: i64,
				i128: i128,
				usize: usize,
				isize: isize,
				f32: f32,
				f64: f64,
				bool: bool,
				char: char,
			}

			let input = Foo {
				u8: 0x01,
				u16: 0x0203,
				u32: 0x04050607,
				u64: 0x08090a0b0c0d0e0f,
				u128: 0x101112131415161718191a1b1c1d1e1f,
				i8: 0x01,
				i16: 0x0203,
				i32: 0x04050607,
				i64: 0x08090a0b0c0d0e0f,
				i128: 0x101112131415161718191a1b1c1d1e1f,
				usize: usize::MAX,
				isize: isize::MAX / 2,
				f32: f32::MAX,
				f64: f64::MAX / 2f64,
				bool: true,
				char: 'c',
			};
			$test_serialize(&input, Test::Primitives, 0);
		}

		#[test]
		fn non_zero_numbers() {
			use std::num;

			#[derive(Serialize, Debug, PartialEq)]
			#[allow(non_snake_case)]
			struct Foo {
				NonZeroU8: num::NonZeroU8,
				NonZeroU16: num::NonZeroU16,
				NonZeroU32: num::NonZeroU32,
				NonZeroU64: num::NonZeroU64,
				NonZeroU128: num::NonZeroU128,
				NonZeroUsize: num::NonZeroUsize,
				NonZeroI8: num::NonZeroI8,
				NonZeroI16: num::NonZeroI16,
				NonZeroI32: num::NonZeroI32,
				NonZeroI64: num::NonZeroI64,
				NonZeroI128: num::NonZeroI128,
				NonZeroIsize: num::NonZeroIsize,
			}

			let input = Foo {
				NonZeroU8: num::NonZeroU8::new(0x01).unwrap(),
				NonZeroU16: num::NonZeroU16::new(0x0203).unwrap(),
				NonZeroU32: num::NonZeroU32::new(0x04050607).unwrap(),
				NonZeroU64: num::NonZeroU64::new(0x08090a0b0c0d0e0f).unwrap(),
				NonZeroU128: num::NonZeroU128::new(0x101112131415161718191a1b1c1d1e1f).unwrap(),
				NonZeroI8: num::NonZeroI8::new(0x01).unwrap(),
				NonZeroI16: num::NonZeroI16::new(0x0203).unwrap(),
				NonZeroI32: num::NonZeroI32::new(0x04050607).unwrap(),
				NonZeroI64: num::NonZeroI64::new(0x08090a0b0c0d0e0f).unwrap(),
				NonZeroI128: num::NonZeroI128::new(0x101112131415161718191a1b1c1d1e1f).unwrap(),
				NonZeroUsize: num::NonZeroUsize::new(usize::MAX).unwrap(),
				NonZeroIsize: num::NonZeroIsize::new(isize::MAX / 2).unwrap(),
			};
			$test_serialize(&input, Test::NonZeroNumbers, 0);
		}

		#[test]
		fn arrays() {
			#[derive(Serialize, Debug, PartialEq)]
			struct Foo {
				empty: [u8; 0],
				single: [u8; 1],
				double: [u16; 2],
				triple: [u32; 3],
			}

			let input = Foo {
				empty: [],
				single: [0x01],
				double: [0x0203, 0x0405],
				triple: [0x06070809, 0x0a0b0c0d, 0x0e0f1011],
			};
			$test_serialize(&input, Test::Arrays, 0);
		}

		#[test]
		fn arrays_of_boxes() {
			#[derive(Serialize, Debug, PartialEq)]
			struct Bar {
				empty: [Box<u8>; 0],
				single: [Box<u8>; 1],
				double: [Box<u16>; 2],
				triple: [Box<u32>; 3],
			}

			let input = Bar {
				empty: [],
				single: [Box::new(0x01)],
				double: [Box::new(0x0203), Box::new(0x0405)],
				triple: [
					Box::new(0x06070809),
					Box::new(0x0a0b0c0d),
					Box::new(0x0e0f1011),
				],
			};
			$test_serialize(&input, Test::ArraysOfBoxes, 0);
		}

		#[test]
		fn tuples() {
			#[derive(Serialize, Debug, PartialEq)]
			struct Foo {
				tup: (u8, u16, u32),
				tup_of_boxes: (Box<u8>, Box<u16>, Box<u32>),
			}

			let input = Foo {
				tup: (0x01, 0x0203, 0x04050607),
				tup_of_boxes: (Box::new(0x08), Box::new(0x090a), Box::new(0x0b0c0d0e)),
			};
			$test_serialize(&input, Test::Tuples, 0);
		}

		#[test]
		fn enum_fieldless() {
			#[derive(Serialize, Debug, PartialEq)]
			enum Foo {
				One,
				Two,
				Three,
			}

			$test_serialize(&Foo::One, Test::EnumFieldless, 0);
			$test_serialize(&Foo::Two, Test::EnumFieldless, 1);
			$test_serialize(&Foo::Three, Test::EnumFieldless, 2);
		}

		#[test]
		fn enum_with_fields() {
			#[derive(Serialize, Debug, PartialEq)]
			enum Foo {
				Bar(Bar),
				Qux(Qux),
			}

			#[derive(Serialize, Debug, PartialEq)]
			struct Bar {
				small: u8,
				big: u32,
			}

			#[derive(Serialize, Debug, PartialEq)]
			enum Qux {
				Small(i8),
				Big(i16),
			}

			$test_serialize(
				&Foo::Bar(Bar {
					small: 0x01,
					big: 0x0203,
				}),
				Test::EnumWithFields,
				0,
			);
			$test_serialize(&Foo::Qux(Qux::Small(0x04)), Test::EnumWithFields, 1);
			$test_serialize(&Foo::Qux(Qux::Big(0x0506)), Test::EnumWithFields, 2);
		}

		#[test]
		fn boxed_primitives() {
			#[derive(Serialize, Debug, PartialEq)]
			struct Foo {
				u8: Box<u8>,
				u16: Box<u16>,
				u32: Box<u32>,
				u64: Box<u64>,
				u128: Box<u128>,
				i8: Box<i8>,
				i16: Box<i16>,
				i32: Box<i32>,
				i64: Box<i64>,
				i128: Box<i128>,
				usize: Box<usize>,
				isize: Box<isize>,
				f32: Box<f32>,
				f64: Box<f64>,
				bool: Box<bool>,
				char: Box<char>,
			}

			let input = Foo {
				u8: Box::new(0x01),
				u16: Box::new(0x0203),
				u32: Box::new(0x04050607),
				u64: Box::new(0x08090a0b0c0d0e0f),
				u128: Box::new(0x101112131415161718191a1b1c1d1e1f),
				i8: Box::new(0x01),
				i16: Box::new(0x0203),
				i32: Box::new(0x04050607),
				i64: Box::new(0x08090a0b0c0d0e0f),
				i128: Box::new(0x101112131415161718191a1b1c1d1e1f),
				usize: Box::new(usize::MAX),
				isize: Box::new(isize::MAX / 2),
				f32: Box::new(f32::MAX),
				f64: Box::new(f64::MAX / 2f64),
				bool: Box::new(true),
				char: Box::new('c'),
			};
			$test_serialize(&input, Test::BoxedPrimitives, 0);
		}

		#[test]
		fn boxed_structs() {
			#[derive(Serialize, Debug, PartialEq)]
			struct Foo {
				bar: Box<Bar>,
				bar2: Box<Bar>,
			}

			#[derive(Serialize, Debug, PartialEq)]
			struct Bar {
				small: u8,
				big: Box<u32>,
			}

			let input = Foo {
				bar: Box::new(Bar {
					small: 0x01,
					big: Box::new(0x02030405),
				}),
				bar2: Box::new(Bar {
					small: 0x01,
					big: Box::new(0x0708090a),
				}),
			};
			$test_serialize(&input, Test::BoxedStructs, 0);
		}

		#[test]
		fn vec_of_primitives() {
			#[derive(Serialize, Debug, PartialEq)]
			struct Foo {
				small: Vec<u8>,
				middle: Vec<u16>,
				big: Vec<u32>,
			}

			$test_serialize(
				&Foo {
					small: Vec::new(),
					middle: Vec::new(),
					big: Vec::new(),
				},
				Test::VecOfPrimitives,
				0,
			);

			$test_serialize(
				&Foo {
					small: vec![0x01],
					middle: vec![0x0203],
					big: vec![0x04050607],
				},
				Test::VecOfPrimitives,
				1,
			);

			$test_serialize(
				&Foo {
					small: vec![0x01, 0x02, 0x03],
					middle: vec![0x0405, 0x0607, 0x0809, 0x0a0b, 0x0c0d],
					big: vec![0x0e0f1012, 0x13141516, 0x1718191a],
				},
				Test::VecOfPrimitives,
				2,
			);
		}

		#[test]
		fn vec_of_vecs() {
			let input: Vec<Vec<u8>> = vec![
				vec![1, 2, 3],
				vec![4, 5, 6, 7, 8, 9],
				vec![10],
				vec![],
				vec![11, 12],
				vec![13, 14, 15, 16],
				vec![],
			];
			$test_serialize(&input, Test::VecOfVecs, 0);
		}

		#[test]
		fn vecs_with_zero_len_zero_capacity() {
			let input = Vec::<u8>::new();
			assert_eq!(input.capacity(), 0);
			$test_serialize(&input, Test::VecsWithZeroLenZeroCapacity, 0);
		}

		#[test]
		fn vecs_with_zero_len_excess_capacity() {
			let input = Vec::<u8>::with_capacity(5);
			assert!(input.capacity() >= 5);
			$test_serialize(&input, Test::VecsWithZeroLenExcessCapacity, 0);
		}

		#[test]
		fn vecs_with_zero_len_excess_capacity2() {
			let mut input = Vec::<u32>::new();
			input.push(1);
			input.pop();
			assert!(input.capacity() > 0);
			$test_serialize(&input, Test::VecsWithZeroLenExcessCapacity2, 0);
		}

		#[test]
		fn vecs_with_excess_capacity() {
			let mut input = Vec::<u8>::with_capacity(5);
			input.push(1);
			assert!(input.capacity() > input.len());
			$test_serialize(&input, Test::VecsWithExcessCapacity, 0);
		}

		#[test]
		fn vecs_with_excess_capacity2() {
			let mut input = vec![0x01020304, 0x05060708, 0x090a0b0c, 0x0d0e0f10];
			input.pop();
			assert!(input.capacity() > 3);
			$test_serialize(&input, Test::VecsWithExcessCapacity2, 0);
		}

		#[test]
		fn strings() {
			$test_serialize(&"abc".to_string(), Test::Strings, 0);
			$test_serialize(&"d".to_string(), Test::Strings, 1);
			$test_serialize(&"efghijkl".to_string(), Test::Strings, 2);
			$test_serialize(&"MNOPQRSTIVWXYZ".to_string(), Test::Strings, 3);
		}

		#[test]
		fn strings_with_zero_len_zero_capacity() {
			let input = "".to_string();
			assert_eq!(input.capacity(), 0);
			$test_serialize(&input, Test::StringsWithZeroLenZeroCapacity, 0);
		}

		#[test]
		fn strings_with_zero_len_excess_capacity() {
			let input = String::with_capacity(5);
			assert!(input.capacity() >= 5);
			$test_serialize(&input, Test::StringsWithZeroLenExcessCapacity, 0);
		}

		#[test]
		fn strings_with_zero_len_excess_capacity2() {
			let mut input = "x".to_string();
			input.pop();
			assert!(input.capacity() >= 1);
			$test_serialize(&input, Test::StringsWithZeroLenExcessCapacity2, 0);
		}

		#[test]
		fn strings_with_excess_capacity() {
			let mut input = String::with_capacity(5);
			input.push('x');
			assert!(input.capacity() > input.len());
			$test_serialize(&input, Test::StringsWithExcessCapacity, 0);
		}

		#[test]
		fn strings_with_excess_capacity2() {
			let mut input = "abcd".to_string();
			input.pop();
			assert!(input.capacity() > 3);
			$test_serialize(&input, Test::StringsWithExcessCapacity2, 0);
		}

		#[test]
		fn options() {
			#[derive(Serialize, Debug, PartialEq)]
			struct Foo {
				bar: Option<Bar>,
				boxed: Option<Box<Bar>>,
				vec: Option<Vec<Bar>>,
				str: Option<String>,
			}

			#[derive(Serialize, Debug, PartialEq)]
			struct Bar {
				small: u8,
				big: u32,
			}

			$test_serialize(
				&Foo {
					bar: None,
					boxed: None,
					vec: None,
					str: None,
				},
				Test::Options,
				0,
			);

			$test_serialize(
				&Foo {
					bar: Some(Bar {
						small: 0x01,
						big: 0x0203,
					}),
					boxed: None,
					vec: None,
					str: Some("".to_string()),
				},
				Test::Options,
				1,
			);

			$test_serialize(
				&Foo {
					bar: None,
					boxed: Some(Box::new(Bar {
						small: 0x04,
						big: 0x0506,
					})),
					vec: Some(vec![
						Bar {
							small: 0x07,
							big: 0x0809,
						},
						Bar {
							small: 0x0a,
							big: 0x0b0c,
						},
						Bar {
							small: 0x0d,
							big: 0x0e0f,
						},
					]),
					str: None,
				},
				Test::Options,
				2,
			);

			$test_serialize(
				&Foo {
					bar: Some(Bar {
						small: 0x10,
						big: 0x1112,
					}),
					boxed: Some(Box::new(Bar {
						small: 0x13,
						big: 0x1415,
					})),
					vec: Some(vec![Bar {
						small: 0x16,
						big: 0x1718,
					}]),
					str: Some("def".to_string()),
				},
				Test::Options,
				3,
			);
		}

		#[test]
		fn structure_where_storage_grows_after_last_pointer_written() {
			#[derive(Serialize, Debug, PartialEq)]
			#[repr(C)]
			struct Foo {
				boxed: Box<u32>,
				vec: Vec<u8>,
				str: String,
				boxed2: Box<u8>,
			}

			let mut input = Foo {
				boxed: Box::new(0x44332211),
				vec: Vec::with_capacity(16),
				str: "well hello! blah blah blah blah blah flaps!".into(),
				boxed2: Box::new(0xff),
			};
			for i in 0..8 {
				input.vec.push((128 + i) as u8);
			}

			$test_serialize(
				&input,
				Test::StructureWhereStorageGrowsAfterLastPointerWritten,
				0,
			);
		}

		#[test]
		fn minecraft_data() {
			let input = generate_minecraft_data();
			$test_serialize(&input, Test::MinecraftData, 0);
		}
	};
}

pub(crate) use tests;
