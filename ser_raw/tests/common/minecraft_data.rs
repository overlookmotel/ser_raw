// Taken from https://github.com/djkoloski/rust_serialization_benchmark

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
	31, 30, 29, 28, 27, 26, 25, 24, 23, 22, 21, 20, 19, 18, 17, 16, 15, 14, 13, 12, 11, 10, 9, 8, 7,
	6, 5, 4, 3, 2, 1, 0,
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
