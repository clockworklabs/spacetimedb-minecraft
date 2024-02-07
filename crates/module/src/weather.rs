use spacetimedb::spacetimedb;
use mc173_module::world::Weather;
use crate::rand::StdbRand;
use crate::StdbTime;

#[spacetimedb(table)]
struct StdbWeather {
    #[primarykey]
    id: u32,
    weather_next_time: u64,
    weather: Weather
}

pub fn init() {
    StdbWeather::insert(StdbWeather {
        id: 0,
        weather_next_time: 0,
        weather: Weather::Clear,
    }).unwrap();
}

#[spacetimedb(reducer)]
pub fn set_weather(weather: Weather) {
    let current_time = StdbTime::filter_by_id(&0).unwrap().time;
    let current_weather = StdbWeather::filter_by_id(&0).unwrap();
    log::info!("Updating weather: {:?} Current time: {} Next update: {}", weather, current_time, current_weather.weather_next_time);

    StdbWeather::update_by_id(&0, StdbWeather {
        id: 0,
        weather_next_time: current_weather.weather_next_time,
        weather
    });
}

pub fn tick_weather() {
    // No weather in the nether.
    // if self.dimension == Dimension::Nether {
        // return;
    // }

    let current_weather = StdbWeather::filter_by_id(&0).unwrap();
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
        StdbWeather::update_by_id(&0, StdbWeather {
            id: 0,
            weather_next_time: current_time.time + delay,
            weather: next_weather,
        });
    }
}
