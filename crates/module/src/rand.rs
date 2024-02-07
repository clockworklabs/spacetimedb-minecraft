use spacetimedb::spacetimedb;
use mc173_module::rand::JavaRandom;

#[spacetimedb(table)]
pub struct StdbRand {
    #[unique]
    pub id: u32,
    pub rand: JavaRandom,
}

pub fn init(nano_time: u128) {
    StdbRand::insert(StdbRand {
        id: 0,
        rand: JavaRandom::new_seeded(nano_time),
    }).unwrap();
}