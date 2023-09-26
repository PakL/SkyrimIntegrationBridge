use warp::{ Filter, reply };
use askama::Template;
use serde::{ Deserialize, Serialize, de::DeserializeOwned };
use std::path::Path;
use std::{ thread, time };
// use serde_json::json;

use std::fs::File;
use std::io::{ Read, Write };

#[derive(Template)]
#[template(path = "ui.html")]
struct UiTemplate {
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct WebhookEventQuery {
	#[serde(rename = "type")]
	event_type: Option<u8>,
	form: String,
	count: u16,
}

const ERROR_RESP: &str = "<html>Oh no!</html>";

fn read_file_with_retry<T: DeserializeOwned>(path: &Path) -> Option<T> {
	let mut result: Option<T> = None;
	let mut retries = 0;
	while result.is_none() {
		if let Ok(mut file) = File::open(path) {
			let mut buf = String::new();
			match file.read_to_string(&mut buf) {
				Ok(_) => {
					match serde_json::from_str::<T>(buf.as_str()) {
						Ok(json) => result = Some(json),
						Err(e) => {
							println!("x Error parsing file {}", path.display());
							println!("x {}", e);
							thread::sleep(time::Duration::from_millis(200));
							retries += 1;
							if retries > 3 {
								break;
							}
						},
					}
				},
				Err(e) => {
					println!("x Error reading file {}", path.display());
					println!("x {}", e);
					thread::sleep(time::Duration::from_millis(200));
					retries += 1;
					if retries > 3 {
						break;
					}
				},
			}
		}
	}
	
	result
}

fn write_file_with_retry<T: Serialize>(path: &Path, data: &T) -> Option<std::io::Error> {
	let mut result: Option<std::io::Error> = Some(std::io::Error::new(std::io::ErrorKind::Other, "Unknown error"));
	let mut retries = 0;

	while result.is_some() {
		if let Ok(mut file) = File::create(path) {
			match file.write_all(serde_json::to_string(data).unwrap().as_bytes()) {
				Ok(_) => result = None,
				Err(e) => {
					println!("x Error writing file {}", path.display());
					println!("x {}", e);
					thread::sleep(time::Duration::from_millis(200));
					retries += 1;
					if retries > 3 {
						break;
					}
				},
			}
		}
	}

	result
}

fn default_response_callback(query: WebhookEventQuery, skyrim_path: String) -> reply::Html<String> {
	if let Some(event_type) = query.event_type {
		if event_type <= 5 {
			let path_buf = Path::new(&skyrim_path).join("events.ptw");
			let path = path_buf.as_path();
			let mut events = read_file_with_retry::<Vec<WebhookEventQuery>>(path).unwrap_or_default();
			events.append(&mut vec![query]);
			write_file_with_retry(path, &events);
		}

		reply::html("ok".to_string())
	} else {
		let tpl = UiTemplate{};
		reply::html(tpl.render().unwrap_or(ERROR_RESP.to_string()))
	}
}

pub async fn start_webinterface(port: u16, skyrim_path: String) {
	let index = warp::path::end()
		.and(warp::query::<WebhookEventQuery>())
		.and(warp::any().map(move || skyrim_path.clone()))
		.map(default_response_callback);


	println!("= Starting server on port {}", port);
	warp::serve(index).run(([127, 0, 0, 1], port)).await;
}