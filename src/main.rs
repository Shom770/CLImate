use chrono::prelude::*;

use cursive::Cursive;
use cursive::views::Dialog;
use rand::{thread_rng, Rng};
use reqwest;
use std::env;
use serde::{Serialize, Deserialize};
use serde_json::{from_str, to_string, Value};
use tokio;

#[derive(Serialize, Deserialize, Debug)]
struct StationMetadata {
    forecast: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CityInfo {
    city: String,
    state: String
}

#[derive(Debug)]
struct ForecastPeriod {
    name: String,
    short_forecast: String,
    temperature: u64,
    wind: String,
    daytime: bool
}

#[derive(Debug)]
struct ObservationData {
    temperature: f64,
    dewpoint: f64,
    description: String,
    wind_speed: f64,
    timestamp: String
}

#[tokio::main]
async fn station_metadata(args: Vec<String>) -> Result<Vec<String>, reqwest::Error> {
    let latitude = str::replace(&args[1], "lat=", "");
    let longitude = str::replace(&args[2], "long=", "");
    let request_url = format!("https://api.weather.gov/points/{lat},{long}", lat=latitude, long=longitude);
    let coords = reqwest::Client::new()
        .get(request_url)
        .header(reqwest::header::USER_AGENT, "2016wxfan@gmail.com")
        .send()
        .await?
        .text()
        .await?;
    
    let coords = String::from(coords);

    let json_res: Value = from_str(&coords).unwrap();
    let properties = json_res
                        .as_object().unwrap()
                        .get(&"properties".to_string()).unwrap();

    let metadata: StationMetadata = from_str(&to_string(properties).unwrap()).unwrap();

    let properties = properties
                        .as_object().unwrap()
                        .get(&"relativeLocation".to_string()).unwrap()
                        .as_object().unwrap()
                        .get(&"properties".to_string()).unwrap();
    
    let city_info: CityInfo = from_str(&to_string(properties).unwrap()).unwrap();

    let stations_link = (&metadata.forecast).strip_suffix("forecast").unwrap().to_owned() + "stations";

    Ok(vec![metadata.forecast, city_info.city, city_info.state, stations_link])
}


#[tokio::main]
async fn forecast(request_url: &String) -> Result<Vec<ForecastPeriod>, reqwest::Error> {
    let forecast_resp = reqwest::Client::new()
        .get(request_url)
        .header(reqwest::header::USER_AGENT, "2016wxfan@gmail.com")
        .header("Feature-Flags", thread_rng().gen_range(100..1000))
        .send()
        .await?
        .text()
        .await?;
    
    let forecast_resp = String::from(forecast_resp);
    let json_forecast: Value = from_str(&forecast_resp).unwrap();

    let periods = json_forecast
        .as_object().unwrap()
        .get(&"properties".to_string()).unwrap()
        .as_object().unwrap()
        .get(&"periods".to_string()).unwrap();

    let mut forecasts: Vec<ForecastPeriod> = vec![];

    for period in periods.as_array().unwrap() {
        let name = period.as_object().unwrap().get("name").unwrap().as_str().unwrap().to_string();
        let temperature = period.as_object().unwrap().get("temperature").unwrap().as_u64().unwrap();
        let wind = period.as_object().unwrap().get("windSpeed").unwrap().as_str().unwrap().to_string();
        let short_forecast = period.as_object().unwrap().get("shortForecast").unwrap().as_str().unwrap().to_string();
        let daytime = period.as_object().unwrap().get("isDaytime").unwrap().as_bool().unwrap();

        forecasts.push(ForecastPeriod{name, short_forecast, temperature, wind, daytime})
    }

    Ok(forecasts)
}

#[tokio::main]
async fn closest_station(request_url: &String) -> Result<String, reqwest::Error> {
    let station_resp = reqwest::Client::new()
        .get(request_url)
        .header(reqwest::header::USER_AGENT, "2016wxfan@gmail.com")
        .header("Feature-Flags", thread_rng().gen_range(100..1000))
        .send()
        .await?
        .text()
        .await?;
    
    let station_resp = String::from(station_resp);
    let json_stations: Value = from_str(&station_resp).unwrap();

    Ok(json_stations["features"][0]["properties"]["stationIdentifier"].as_str().unwrap().to_string())
}

#[tokio::main]
async fn station_observations(station_url: String) -> Result<ObservationData, reqwest::Error> {
    let observations_resp = reqwest::Client::new()
        .get(station_url)
        .header(reqwest::header::USER_AGENT, "2016wxfan@gmail.com")
        .header("Feature-Flags", thread_rng().gen_range(100..1000))
        .send()
        .await?
        .text()
        .await?;

    let observations_resp = String::from(observations_resp);
    let json_observations: Value = from_str(&observations_resp).unwrap();

    let properties = &json_observations["properties"];

    let temperature = if properties["temperature"]["unitCode"].as_str().unwrap().ends_with("degC") {
                        (properties["temperature"]["value"].as_f64().unwrap() as f64) * ((9 / 5) as f64) + 32.0
                    }
                    else {
                        properties["temperature"]["value"].as_f64().unwrap()
                    };

    let dewpoint = if properties["dewpoint"]["unitCode"].as_str().unwrap().ends_with("degC") {
                        (properties["dewpoint"]["value"].as_f64().unwrap() as f64) * ((9 / 5) as f64) + 32.0
                    }
                    else {
                        properties["dewpoint"]["value"].as_f64().unwrap()
                    };

    let description = properties["textDescription"].as_str().unwrap().to_string();
    
    let wind_speed = if properties["windSpeed"]["unitCode"].as_str().unwrap().ends_with("km_h-1") {
                        (properties["windSpeed"]["value"].as_f64().unwrap() as f64) * 0.6213
                    }
                    else {
                        properties["windSpeed"]["value"].as_f64().unwrap()
                    };

    let timestamp = properties["timestamp"].as_str().unwrap().to_string();
    
    Ok(ObservationData{temperature, dewpoint, description, wind_speed, timestamp})
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let coordinates = station_metadata(args).unwrap();

    let station_name = closest_station(&coordinates[3]).unwrap();

    let observation = station_observations(format!("https://api.weather.gov/stations/{station}/observations/latest", station=station_name)).unwrap();

    let _forecasts = forecast(&coordinates[0]).unwrap();

    let last_observation = DateTime::<FixedOffset>::parse_from_rfc3339(&observation.timestamp).unwrap().with_timezone(&Utc);
    let current_time = Utc::now();

    let difference = current_time - last_observation;

    let formatted_difference = if difference.num_seconds() < 60 {
                                String::from("just now.")
                            }
                            else {
                                let diff_string = difference.num_minutes().to_string();
                                diff_string + " minutes ago."
                            };

    let mut siv = cursive::default();

    siv.add_layer(Dialog::text(format!("Temperature: {}° F\nDewpoint: {}° F\nConditions: {}\nWind Speed: {:.2} MPH", observation.temperature, observation.dewpoint, observation.description, observation.wind_speed.round()))
        .title(format!("Conditions at {}, {} - {}", &coordinates[1], &coordinates[2], formatted_difference))
    );
	siv.run();
}
