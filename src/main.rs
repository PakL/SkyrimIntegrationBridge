use serde::{ Serialize, Deserialize };
use tokio::join;

use std::fs::File;
use std::io::Read;

mod webinterface;

#[derive(Serialize, Deserialize, Default)]
struct BridgeConfig {
	port: u16,
	skyrimpath: String,
}

#[tokio::main]
async fn main() {
	println!("= Loading config file");

	let config_file_result = File::open("config.json");
	if config_file_result.is_err() {
		println!("x Could not open config file");
		println!("x {}", config_file_result.err().unwrap());
		return;
	}

	let mut config_file = config_file_result.unwrap();

	let mut json_buffer: String = String::new();
	config_file.read_to_string(&mut json_buffer).unwrap();

	drop(config_file); // Dropping to close file

	match serde_json::from_str::<BridgeConfig>(json_buffer.as_str()) {
		Ok(config) => {
			println!("= Skyrim path: {}", config.skyrimpath);

			join!(webinterface::start_webinterface(config.port, config.skyrimpath));
		},
		Err(e) => {
			println!("x Invalid config file");
			println!("x {}", e);
		},
	}
}
