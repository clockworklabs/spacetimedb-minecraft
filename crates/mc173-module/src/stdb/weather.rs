use spacetimedb::{log, spacetimedb};
use crate::stdb::chunk::StdbTime;
use crate::stdb::rand::StdbRand;
use crate::world::{StdbWorld, Weather, DIMENSION_NETHER, DIMENSION_OVERWORLD};

#[spacetimedb(table(public))]
pub struct StdbWeather {
    #[primarykey]
    pub dimension_id: i32,
    pub weather_next_time: u64,
    pub weather: Weather
}

pub fn init() {
    StdbWeather::insert(StdbWeather {
        dimension_id: DIMENSION_OVERWORLD,
        weather_next_time: 0,
        weather: Weather::Clear,
    }).unwrap();
    // NOTE: No weather for the nether!
}

#[spacetimedb(reducer)]
pub fn set_weather(new_weather: Weather, dimension_id: i32) {
    let world = StdbWorld::filter_by_dimension_id(&dimension_id).unwrap();
    let current_time = StdbTime::filter_by_id(&0).unwrap().time;
    let current_weather = StdbWeather::filter_by_dimension_id(&dimension_id).expect(format!("No weather for dimension: {}", dimension_id).as_str());
    log::info!("Updating weather: {:?} Current time: {} Next update: {}", new_weather, current_time, current_weather.weather_next_time);
    world.set_weather(new_weather);
}

pub fn tick_weather(dimension_id: i32) {
    // No weather in the nether.
    if dimension_id == DIMENSION_NETHER {
        return;
    }

    let current_weather = StdbWeather::filter_by_dimension_id(&dimension_id).unwrap();
    let current_time = StdbTime::filter_by_id(&0).unwrap();
    let mut current_rand = StdbRand::filter_by_id(&0).unwrap();
    let mut next_weather = current_weather.weather;

    // When it's time to recompute weather.
    if current_time.time >= current_weather.weather_next_time {
        // Don't update weather on first world tick.
        if current_time.time != 0 {
            next_weather = match current_weather.weather {
                Weather::Clear => current_rand.rand.next_choice(&[Weather::Rain, Weather::Thunder]),
                _ => current_rand.rand.next_choice(&[current_weather.weather, Weather::Clear]),
            };
        }

        let bound = if current_weather.weather == Weather::Clear {
            168000
        } else {
            12000
        };
        let delay = current_rand.rand.next_int_bounded(bound) as u64 + 12000;
        // self.weather_next_time = self.get_time() + delay;
        log::info!("Updating weather: {:?}", next_weather);
        StdbWeather::update_by_dimension_id(&dimension_id, StdbWeather {
            dimension_id: 0,
            weather_next_time: current_time.time + delay,
            weather: next_weather,
        });
    }
}