use warp::{ Filter, reply };
use askama::Template;
use serde::{ Deserialize, Serialize, de::DeserializeOwned };
use std::path::Path;
use std::sync::Mutex;
use std::{ thread, time };
use std::collections::HashMap;
use std::fs::File;
use std::io::{ Read, Write };
use serde_json::json;

use super::{ formsearch, formsearch::{ Form, FormIndex } };

#[derive(Template)]
#[template(path = "ui.html")]
struct UiTemplate {
}

#[derive(Template)]
#[template(path = "search_results.html")]
struct SearchResultsTemplate {
	results: Vec<Form>,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct WebhookEventQuery {
	#[serde(rename = "type")]
	event_type: Option<u8>,
	form: String,
	count: u16,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct SearchQuery {
	query: String,
}

const ERROR_RESP: &str = "<html>Oh no!</html>";

fn prepare_formid(form: String) -> String {
	let mut splits: Vec<&str> = form.split("|").collect();
	if splits.len() == 1 {
		splits.insert(0, "Skyrim.esm");
	}
	if splits.len() == 2 {
		splits.insert(0, "__formData");
	}
	if *splits.get(0).unwrap() != "__formData" {
		splits.splice(0..1, ["__formData"]);
	}

	if splits.len() != 3 {
		return String::new();
	}

	let formid = splits.get(2).unwrap().to_string();
	if formid.len() > 8 {
		return String::new();
	}

	let chars = formid.chars();
	let mut has_prefix = true;
	for (i, c) in chars.enumerate() {
		if has_prefix && i == 0 {
			if c == '0' {
				continue;
			} else {
				has_prefix = false;
			}
		}
		if has_prefix && i == 1 {
			if c == 'x' || c == 'X' {
				continue;
			} else {
				has_prefix = false;
			}
		}

		if !c.is_ascii_hexdigit() {
			return String::new();
		}
	}

	let new_formid = format!("0x{}", formid);
	if !has_prefix {
		splits.splice(2..3, [new_formid.as_str()]);
	}

	splits.join("|")
}

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

fn default_response_callback(query: WebhookEventQuery, skyrim_path: String) -> reply::WithStatus<reply::Html<String>> {
	if let Some(event_type) = query.event_type {
		println!("> Received event {}", json!(query));
		if event_type <= 5 {
			let path_buf = Path::new(&skyrim_path).join("events.ptw");
			let path = path_buf.as_path();
			let mut events = read_file_with_retry::<Vec<WebhookEventQuery>>(path).unwrap_or_default();
			let new_event = WebhookEventQuery {
				event_type: Some(event_type),
				form: prepare_formid(query.form),
				count: if query.count < 1 { 1 } else { query.count },
			};
			if new_event.form.is_empty() {
				return reply::with_status(reply::html("invalid form".to_string()), warp::http::StatusCode::BAD_REQUEST);
			}

			let event_json = json!(new_event);
			events.append(&mut vec![new_event]);
			let write_error = write_file_with_retry(path, &events);

			let (message, status) = match write_error {
				Some(e) => (e.to_string(), warp::http::StatusCode::INTERNAL_SERVER_ERROR),
				None	=> {
					println!("< Wrote event {}", event_json);
					("ok".to_string(), warp::http::StatusCode::OK)
				},
			};
			reply::with_status(reply::html(message), status)
		} else {
			println!("x Invalid form");
			reply::with_status(reply::html("invalid type".to_string()), warp::http::StatusCode::BAD_REQUEST)
		}
	} else {
		let tpl = UiTemplate{};
		reply::with_status(reply::html(tpl.render().unwrap_or(ERROR_RESP.to_string())), warp::http::StatusCode::OK)
	}
}

static FORM_INDEX: Mutex<Option<HashMap<String, FormIndex>>> = Mutex::new(None);

fn search_response_callback(query: SearchQuery) -> reply::WithStatus<reply::Html<String>> {
	if query.query.is_empty() {
		return reply::with_status(reply::html("".to_string()), warp::http::StatusCode::OK);
	}

	println!("> Searching for {}", query.query);
	let lock = FORM_INDEX.lock().unwrap();
	let index = lock.as_ref().unwrap();
	let forms = formsearch::find_forms(&index, query.query.as_str(), vec![], vec![]);
	println!("< {} results", forms.len());
	let tpl = SearchResultsTemplate{ results: forms };
	reply::with_status(reply::html(tpl.render().unwrap_or(ERROR_RESP.to_string())), warp::http::StatusCode::OK)
}


pub async fn start_webinterface(port: u16, skyrim_path: String) {
	let index = warp::path::end()
		.and(warp::query::<WebhookEventQuery>())
		.and(warp::any().map(move || skyrim_path.clone()))
		.map(default_response_callback);

	
	let mut index_lock = FORM_INDEX.lock().unwrap();
	*index_lock = Some(formsearch::build_index());
	drop(index_lock);

	let search = warp::path("search")
		.and(warp::query::<SearchQuery>())
		.map(search_response_callback);

	let routes = index.or(search);

	println!("= Starting server on port {}", port);
	warp::serve(routes).run(([127, 0, 0, 1], port)).await;
}