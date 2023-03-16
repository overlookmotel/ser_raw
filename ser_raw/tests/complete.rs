use std::{fmt::Debug, mem};

use ser_raw::{
	storage::{aligned_max_capacity, AlignedVec, ContiguousStorage},
	CompleteSerializer, InstantiableSerializer, Serialize, Serializer,
};

const MAX_CAPACITY: usize = aligned_max_capacity(16);
const PTR_SIZE: usize = mem::size_of::<usize>();

type AlVec = AlignedVec<16, 8, 8, MAX_CAPACITY>;
type Ser = CompleteSerializer<16, 8, 8, MAX_CAPACITY, AlVec>;

fn test_serialize<T>(input: &T)
where T: Serialize<Ser> + Debug + PartialEq {
	let storage = serialize(input);
	let output: &T = deserialize(&storage);
	assert_eq!(input, output);
}

fn serialize<T: Serialize<Ser>>(value: &T) -> AlVec {
	let mut ser = Ser::new();
	ser.serialize_value(value);
	ser.finalize()
}

fn deserialize<T>(storage: &AlVec) -> &T {
	unsafe { &*storage.as_ptr().cast() }
}

// NB: Cannot easily test for error if try to serialize a type with alignment
// greater than the serializer's `MAX_VALUE_ALIGNMENT`, because it's an error at
// compile time, not runtime.

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
	test_serialize(&input);
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
	test_serialize(&input);
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
	test_serialize(&input);

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
	test_serialize(&input);
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
	test_serialize(&input);
}

#[test]
fn enum_fieldless() {
	#[derive(Serialize, Debug, PartialEq)]
	enum Foo {
		One,
		Two,
		Three,
	}

	test_serialize(&Foo::One);
	test_serialize(&Foo::Two);
	test_serialize(&Foo::Three);
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

	test_serialize(&Foo::Bar(Bar {
		small: 0x01,
		big: 0x0203,
	}));
	test_serialize(&Foo::Qux(Qux::Small(0x04)));
	test_serialize(&Foo::Qux(Qux::Big(0x0506)));
}

#[test]
fn boxed_primitive() {
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
	test_serialize(&input);
}

#[test]
fn boxed_struct() {
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
	test_serialize(&input);
}

#[test]
fn vec_of_primitives() {
	#[derive(Serialize, Debug, PartialEq)]
	struct Foo {
		small: Vec<u8>,
		middle: Vec<u16>,
		big: Vec<u32>,
	}

	test_serialize(&Foo {
		small: Vec::new(),
		middle: Vec::new(),
		big: Vec::new(),
	});

	test_serialize(&Foo {
		small: vec![0x01],
		middle: vec![0x0203],
		big: vec![0x04050607],
	});

	test_serialize(&Foo {
		small: vec![0x01, 0x02, 0x03],
		middle: vec![0x0405, 0x0607, 0x0809, 0x0a0b, 0x0c0d],
		big: vec![0x0e0f1012, 0x13141516, 0x1718191a],
	});
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
	test_serialize(&input);
}

#[test]
fn vec_with_zero_len() {
	// Zero length vecs are reduced to 0 capacity and dangling pointer

	// 0 capacity originally
	let input = Vec::<u8>::new();
	assert_eq!(input.capacity(), 0);
	let storage = serialize(&input);

	assert_eq!(storage.as_slice().len(), PTR_SIZE * 3);
	let parts: &[usize; 3] = unsafe { &*storage.as_ptr().cast() };
	assert_eq!(parts, &[0, 1, 0]);

	let output: &Vec<u8> = deserialize(&storage);
	assert_eq!(&input, output);
	assert_eq!(output.len(), 0);
	assert_eq!(output.capacity(), 0);

	// Excess capacity
	let input = Vec::<u8>::with_capacity(5);
	assert!(input.capacity() >= 5);
	let storage = serialize(&input);

	assert_eq!(storage.as_slice().len(), PTR_SIZE * 3);
	let parts: &[usize; 3] = unsafe { &*storage.as_ptr().cast() };
	assert_eq!(parts, &[0, 1, 0]);

	let output: &Vec<u8> = deserialize(&storage);
	assert_eq!(&input, output);
	assert_eq!(output.len(), 0);
	assert_eq!(output.capacity(), 0);

	// Excess capacity
	let mut input = Vec::<u32>::new();
	input.push(1);
	input.pop();
	let storage = serialize(&input);

	assert_eq!(storage.as_slice().len(), PTR_SIZE * 3);
	let parts: &[usize; 3] = unsafe { &*storage.as_ptr().cast() };
	assert_eq!(parts, &[0, 4, 0]);

	let output: &Vec<u32> = deserialize(&storage);
	assert_eq!(&input, output);
	assert_eq!(output.len(), 0);
	assert_eq!(output.capacity(), 0);
}

#[test]
fn vec_with_spare_capacity() {
	// Vecs with spare capacity are shrunk to fit
	let mut input = Vec::<u8>::with_capacity(5);
	input.push(1);
	let storage = serialize(&input);

	let output: &Vec<u8> = deserialize(&storage);
	assert_eq!(&input, output);
	assert_eq!(output.len(), 1);
	assert_eq!(output.capacity(), 1);

	let mut input = vec![0x01020304, 0x05060708, 0x090a0b0c, 0x0d0e0f10];
	input.pop();
	assert!(input.capacity() > 3);
	let storage = serialize(&input);

	let output: &Vec<u32> = deserialize(&storage);
	assert_eq!(&input, output);
	assert_eq!(output.len(), 3);
	assert_eq!(output.capacity(), 3);
}

#[test]
fn string() {
	test_serialize(&"abc".to_string());
	test_serialize(&"d".to_string());
	test_serialize(&"efghijkl".to_string());
	test_serialize(&"MNOPQRSTIVWXYZ".to_string());
}

#[test]
fn string_with_zero_len() {
	// Zero length strings are reduced to 0 capacity and dangling pointer

	// 0 capacity originally
	let input = "".to_string();
	assert_eq!(input.capacity(), 0);
	let storage = serialize(&input);

	assert_eq!(storage.as_slice().len(), PTR_SIZE * 3);
	let parts: &[usize; 3] = unsafe { &*storage.as_ptr().cast() };
	assert_eq!(parts, &[0, 1, 0]);

	let output: &String = deserialize(&storage);
	assert_eq!(&input, output);
	assert_eq!(output.len(), 0);
	assert_eq!(output.capacity(), 0);

	// Excess capacity
	let input = String::with_capacity(5);
	let storage = serialize(&input);

	assert_eq!(storage.as_slice().len(), PTR_SIZE * 3);
	let parts: &[usize; 3] = unsafe { &*storage.as_ptr().cast() };
	assert_eq!(parts, &[0, 1, 0]);

	let output: &String = deserialize(&storage);
	assert_eq!(&input, output);
	assert_eq!(output.len(), 0);
	assert_eq!(output.capacity(), 0);

	// Excess capacity
	let mut input = "x".to_string();
	input.pop();
	let storage = serialize(&input);

	assert_eq!(storage.as_slice().len(), PTR_SIZE * 3);
	let parts: &[usize; 3] = unsafe { &*storage.as_ptr().cast() };
	assert_eq!(parts, &[0, 1, 0]);

	let output: &String = deserialize(&storage);
	assert_eq!(&input, output);
	assert_eq!(output.len(), 0);
	assert_eq!(output.capacity(), 0);
}

#[test]
fn option() {
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

	test_serialize(&Foo {
		bar: None,
		boxed: None,
		vec: None,
		str: None,
	});

	test_serialize(&Foo {
		bar: Some(Bar {
			small: 0x01,
			big: 0x0203,
		}),
		boxed: None,
		vec: None,
		str: Some("".to_string()),
	});

	test_serialize(&Foo {
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
	});

	test_serialize(&Foo {
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
	});
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

	test_serialize(&input);
}

#[test]
fn large_structure() {
	let input = minecraft_data::generate_data();
	test_serialize(&input);
}

// Taken from https://github.com/djkoloski/rust_serialization_benchmark
#[cfg(test)]
mod minecraft_data {
	use std::{mem, ops};

	use rand::Rng;
	use rand_pcg::Lcg64Xsh32;
	use ser_raw::Serialize;

	#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
	pub enum GameType {
		Survival,
		Creative,
		Adventure,
		Spectator,
	}

	impl Generate for GameType {
		fn generate<R: Rng>(rand: &mut R) -> Self {
			match rand.gen_range(0..4) {
				0 => GameType::Survival,
				1 => GameType::Creative,
				2 => GameType::Adventure,
				3 => GameType::Spectator,
				_ => unreachable!(),
			}
		}
	}

	#[derive(Serialize, Clone, Debug, PartialEq)]
	pub struct Item {
		pub count: i8,
		pub slot: u8,
		pub id: String,
	}

	impl Generate for Item {
		fn generate<R: Rng>(rng: &mut R) -> Self {
			const IDS: [&str; 8] = [
				"dirt",
				"stone",
				"pickaxe",
				"sand",
				"gravel",
				"shovel",
				"chestplate",
				"steak",
			];
			Self {
				count: rng.gen(),
				slot: rng.gen(),
				id: IDS[rng.gen_range(0..IDS.len())].to_string(),
			}
		}
	}

	#[derive(Serialize, Clone, Copy, Debug, PartialEq)]
	pub struct Abilities {
		pub walk_speed: f32,
		pub fly_speed: f32,
		pub may_fly: bool,
		pub flying: bool,
		pub invulnerable: bool,
		pub may_build: bool,
		pub instabuild: bool,
	}

	impl Generate for Abilities {
		fn generate<R: Rng>(rng: &mut R) -> Self {
			Self {
				walk_speed: rng.gen(),
				fly_speed: rng.gen(),
				may_fly: rng.gen_bool(0.5),
				flying: rng.gen_bool(0.5),
				invulnerable: rng.gen_bool(0.5),
				may_build: rng.gen_bool(0.5),
				instabuild: rng.gen_bool(0.5),
			}
		}
	}

	#[derive(Serialize, Clone, Debug, PartialEq)]
	pub struct Entity {
		pub id: String,
		pub pos: (f64, f64, f64),
		pub motion: (f64, f64, f64),
		pub rotation: (f32, f32),
		pub fall_distance: f32,
		pub fire: u16,
		pub air: u16,
		pub on_ground: bool,
		pub no_gravity: bool,
		pub invulnerable: bool,
		pub portal_cooldown: i32,
		pub uuid: [u32; 4],
		pub custom_name: Option<String>,
		pub custom_name_visible: bool,
		pub silent: bool,
		pub glowing: bool,
	}

	impl Generate for Entity {
		fn generate<R: Rng>(rng: &mut R) -> Self {
			const IDS: [&str; 8] = [
				"cow", "sheep", "zombie", "skeleton", "spider", "creeper", "parrot", "bee",
			];
			const CUSTOM_NAMES: [&str; 8] = [
				"rainbow", "princess", "steve", "johnny", "missy", "coward", "fairy", "howard",
			];

			Self {
				id: IDS[rng.gen_range(0..IDS.len())].to_string(),
				pos: <(f64, f64, f64) as Generate>::generate(rng),
				motion: <(f64, f64, f64) as Generate>::generate(rng),
				rotation: <(f32, f32) as Generate>::generate(rng),
				fall_distance: rng.gen(),
				fire: rng.gen(),
				air: rng.gen(),
				on_ground: rng.gen_bool(0.5),
				no_gravity: rng.gen_bool(0.5),
				invulnerable: rng.gen_bool(0.5),
				portal_cooldown: rng.gen(),
				uuid: <[u32; 4] as Generate>::generate(rng),
				custom_name: <Option<()> as Generate>::generate(rng)
					.map(|_| CUSTOM_NAMES[rng.gen_range(0..CUSTOM_NAMES.len())].to_string()),
				custom_name_visible: rng.gen_bool(0.5),
				silent: rng.gen_bool(0.5),
				glowing: rng.gen_bool(0.5),
			}
		}
	}

	#[derive(Serialize, Clone, Debug, PartialEq)]
	pub struct RecipeBook {
		pub recipes: Vec<String>,
		pub to_be_displayed: Vec<String>,
		pub is_filtering_craftable: bool,
		pub is_gui_open: bool,
		pub is_furnace_filtering_craftable: bool,
		pub is_furnace_gui_open: bool,
		pub is_blasting_furnace_filtering_craftable: bool,
		pub is_blasting_furnace_gui_open: bool,
		pub is_smoker_filtering_craftable: bool,
		pub is_smoker_gui_open: bool,
	}

	impl Generate for RecipeBook {
		fn generate<R: Rng>(rng: &mut R) -> Self {
			const RECIPES: [&str; 8] = [
				"pickaxe",
				"torch",
				"bow",
				"crafting table",
				"furnace",
				"shears",
				"arrow",
				"tnt",
			];
			const MAX_RECIPES: usize = 30;
			const MAX_DISPLAYED_RECIPES: usize = 10;
			Self {
				recipes: generate_vec::<_, ()>(rng, 0..MAX_RECIPES)
					.iter()
					.map(|_| RECIPES[rng.gen_range(0..RECIPES.len())].to_string())
					.collect(),
				to_be_displayed: generate_vec::<_, ()>(rng, 0..MAX_DISPLAYED_RECIPES)
					.iter()
					.map(|_| RECIPES[rng.gen_range(0..RECIPES.len())].to_string())
					.collect(),
				is_filtering_craftable: rng.gen_bool(0.5),
				is_gui_open: rng.gen_bool(0.5),
				is_furnace_filtering_craftable: rng.gen_bool(0.5),
				is_furnace_gui_open: rng.gen_bool(0.5),
				is_blasting_furnace_filtering_craftable: rng.gen_bool(0.5),
				is_blasting_furnace_gui_open: rng.gen_bool(0.5),
				is_smoker_filtering_craftable: rng.gen_bool(0.5),
				is_smoker_gui_open: rng.gen_bool(0.5),
			}
		}
	}

	#[derive(Serialize, Clone, Debug, PartialEq)]
	pub struct Player {
		pub game_type: GameType,
		pub previous_game_type: GameType,
		pub score: i64,
		pub dimension: String,
		pub selected_item_slot: u32,
		pub selected_item: Item,
		pub spawn_dimension: Option<String>,
		pub spawn_x: i64,
		pub spawn_y: i64,
		pub spawn_z: i64,
		pub spawn_forced: Option<bool>,
		pub sleep_timer: u16,
		pub food_exhaustion_level: f32,
		pub food_saturation_level: f32,
		pub food_tick_timer: u32,
		pub xp_level: u32,
		pub xp_p: f32,
		pub xp_total: i32,
		pub xp_seed: i32,
		pub inventory: Vec<Item>,
		pub ender_items: Vec<Item>,
		pub abilities: Abilities,
		pub entered_nether_position: Option<(f64, f64, f64)>,
		pub root_vehicle: Option<([u32; 4], Entity)>,
		pub shoulder_entity_left: Option<Entity>,
		pub shoulder_entity_right: Option<Entity>,
		pub seen_credits: bool,
		pub recipe_book: RecipeBook,
	}

	impl Generate for Player {
		fn generate<R: Rng>(rng: &mut R) -> Self {
			const DIMENSIONS: [&str; 3] = ["overworld", "nether", "end"];
			const MAX_ITEMS: usize = 40;
			const MAX_ENDER_ITEMS: usize = 27;
			Self {
				game_type: GameType::generate(rng),
				previous_game_type: GameType::generate(rng),
				score: rng.gen(),
				dimension: DIMENSIONS[rng.gen_range(0..DIMENSIONS.len())].to_string(),
				selected_item_slot: rng.gen(),
				selected_item: Item::generate(rng),
				spawn_dimension: <Option<()> as Generate>::generate(rng)
					.map(|_| DIMENSIONS[rng.gen_range(0..DIMENSIONS.len())].to_string()),
				spawn_x: rng.gen(),
				spawn_y: rng.gen(),
				spawn_z: rng.gen(),
				spawn_forced: <Option<bool> as Generate>::generate(rng),
				sleep_timer: rng.gen(),
				food_exhaustion_level: rng.gen(),
				food_saturation_level: rng.gen(),
				food_tick_timer: rng.gen(),
				xp_level: rng.gen(),
				xp_p: rng.gen(),
				xp_total: rng.gen(),
				xp_seed: rng.gen(),
				inventory: generate_vec(rng, 0..MAX_ITEMS),
				ender_items: generate_vec(rng, 0..MAX_ENDER_ITEMS),
				abilities: Abilities::generate(rng),
				entered_nether_position: <Option<(f64, f64, f64)> as Generate>::generate(rng),
				root_vehicle: <Option<([u32; 4], Entity)> as Generate>::generate(rng),
				shoulder_entity_left: <Option<Entity> as Generate>::generate(rng),
				shoulder_entity_right: <Option<Entity> as Generate>::generate(rng),
				seen_credits: rng.gen_bool(0.5),
				recipe_book: RecipeBook::generate(rng),
			}
		}
	}

	#[derive(Serialize, Clone, Debug, PartialEq)]
	pub struct Players {
		pub players: Vec<Player>,
	}

	pub trait Generate {
		fn generate<R: Rng>(rng: &mut R) -> Self;
	}

	impl Generate for () {
		fn generate<R: Rng>(_: &mut R) -> Self {}
	}

	impl Generate for bool {
		fn generate<R: Rng>(rng: &mut R) -> Self {
			rng.gen_bool(0.5)
		}
	}

	macro_rules! impl_generate {
		($ty:ty) => {
			impl Generate for $ty {
				fn generate<R: Rng>(rng: &mut R) -> Self {
					rng.gen()
				}
			}
		};
	}

	impl_generate!(u8);
	impl_generate!(u16);
	impl_generate!(u32);
	impl_generate!(u64);
	impl_generate!(u128);
	impl_generate!(usize);
	impl_generate!(i8);
	impl_generate!(i16);
	impl_generate!(i32);
	impl_generate!(i64);
	impl_generate!(i128);
	impl_generate!(isize);
	impl_generate!(f32);
	impl_generate!(f64);

	macro_rules! impl_tuple {
    () => {};
    ($first:ident, $($rest:ident,)*) => {
        impl<$first: Generate, $($rest: Generate,)*> Generate for ($first, $($rest,)*) {
            fn generate<R: Rng>(rng: &mut R) -> Self {
                ($first::generate(rng), $($rest::generate(rng),)*)
            }
        }

        impl_tuple!($($rest,)*);
    };
	}

	impl_tuple!(T0, T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11,);

	macro_rules! impl_array {
    () => {};
    ($len:literal, $($rest:literal,)*) => {
        impl<T: Generate> Generate for [T; $len] {
            fn generate<R: Rng>(rng: &mut R) -> Self {
                let mut result = mem::MaybeUninit::<Self>::uninit();
                let result_ptr = result.as_mut_ptr().cast::<T>();
                for i in 0..$len {
                    unsafe {
                        result_ptr.add(i).write(T::generate(rng));
                    }
                }
                unsafe {
                    result.assume_init()
                }
            }
        }

        impl_array!($($rest,)*);
    }
	}

	impl_array!(
		31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8,
		7, 6, 5, 4, 3, 2, 1, 0,
	);

	impl<T: Generate> Generate for Option<T> {
		fn generate<R: Rng>(rng: &mut R) -> Self {
			if rng.gen_bool(0.5) {
				Some(T::generate(rng))
			} else {
				None
			}
		}
	}

	fn generate_vec<R: Rng, T: Generate>(rng: &mut R, range: ops::Range<usize>) -> Vec<T> {
		let len = rng.gen_range(range);
		let mut result = Vec::with_capacity(len);
		for _ in 0..len {
			result.push(T::generate(rng));
		}
		result
	}

	pub fn generate_data() -> Players {
		const STATE: u64 = 3141592653;
		const STREAM: u64 = 5897932384;

		let mut rng = Lcg64Xsh32::new(STATE, STREAM);

		const PLAYERS: usize = 500;
		Players {
			players: generate_vec::<_, Player>(&mut rng, PLAYERS..PLAYERS + 1),
		}
	}
}
