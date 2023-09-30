use warp::{ Filter, reply };
use askama::Template;
use serde::{ Deserialize, Serialize, de::DeserializeOwned };
use std::path::Path;
use std::sync::Mutex;
use std::collections::HashMap;
use std::{ thread, time };
use std::fs::File;
use std::io::{ Read, Write };
use serde_json::json;
use regex::Regex;

use super::aliases;
use super::{ formsearch, formsearch::{ Form, FormIndex } };

#[derive(Template)]
#[template(path = "ui.html")]
struct UiTemplate {
	aliases: Vec<aliases::Alias>,
	alias_write_error: Option<String>,
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
	alias_group: String,
	count: u16,
}

#[derive(Serialize, Deserialize)]
struct WebhookNamedEventQuery {
	form: String,
	#[serde(default, skip_serializing)]
	alias_group: String,
	name: String,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(default)]
struct SearchQuery {
	query: String,
}

const ERROR_RESP: &str = "<html>Oh no!</html>";

fn prepare_formid(form: String, for_jcontainers: bool) -> String {
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

	let formid = splits.get(2).unwrap().to_ascii_uppercase().to_string();
	if formid.len() > 8 || formid.is_empty() {
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

	let mut new_formid = format!("0x{}", formid);
	if has_prefix {
		new_formid = formid.replace("X", "x");
	}
	splits.splice(2..3, [new_formid.as_str()]);

	if !for_jcontainers {
		splits.remove(0);
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

fn formid_or_alias(form: String, alias_group: String) -> String {
	let mut form_id;
	if alias_group.len() > 0 {
		let mut form_from_alias = String::new();
		if let Some(alias) = aliases::get_alias(&form) {
			if alias_group == "*" {
				form_from_alias = alias.form;
			} else {
				let groups = alias.filter_group.split(',');
				for group in groups {
					if group == alias_group {
						form_from_alias = alias.form;
						break;
					}
				}
			}
		}

		form_id = prepare_formid(form_from_alias, true);
	} else {
		form_id = prepare_formid(form.clone(), true);
		if form_id.is_empty() {
			if let Some(alias) = aliases::get_alias(&form) {
				form_id = prepare_formid(alias.form, true);
			}
		}
	}

	form_id
}

fn default_response_callback(query: WebhookEventQuery, skyrim_path: String, alias_form: HashMap<String, String>) -> reply::WithStatus<reply::Html<String>> {
	if let Some(event_type) = query.event_type {
		println!("> Received event {}", json!(query));
		if event_type <= 5 {
			let form = formid_or_alias(query.form, query.alias_group);
			if form.is_empty() {
				println!("x Invalid form");
				return reply::with_status(reply::html("invalid form".to_string()), warp::http::StatusCode::BAD_REQUEST);
			}

			let path_buf = Path::new(&skyrim_path).join("events.ptw");
			let path = path_buf.as_path();
			let mut events = read_file_with_retry::<Vec<WebhookEventQuery>>(path).unwrap_or_default();
			let new_event = WebhookEventQuery {
				event_type: Some(event_type),
				form,
				alias_group: String::new(),
				count: if query.count < 1 { 1 } else { query.count },
			};

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
			println!("x Invalid event type");
			reply::with_status(reply::html("invalid type".to_string()), warp::http::StatusCode::BAD_REQUEST)
		}
	} else {
		let mut alias_write_error: Option<String> = None;
		if alias_form.contains_key("alias_new") {
			println!("> Incoming alias save");
			let allow_letters = Regex::new(r"[^a-z,]").unwrap();
			let mut new_aliases: Vec<aliases::Alias> = vec![];
			for (key, value) in alias_form.iter() {
				if key.starts_with("alias_") {
					let alias = value.replace(" ", "");
					if alias.is_empty() {
						continue;
					}

					let suffix = key.trim_start_matches("alias_");
					let mut form = alias_form.get(&format!("form_{}", suffix)).map_or(String::new(), |v| v.clone());
					let form_prep = prepare_formid(form.clone(), false);
					if form_prep.is_empty() {
						println!("x Invalid form for alias {}", alias);
						alias_write_error = Some(format!("Invalid form for alias {}", alias));
					} else {
						form = form_prep;
					}
					let mut filter_group = alias_form.get(&format!("group_{}", suffix)).map_or(String::new(), |v| v.clone());
					filter_group = allow_letters.replace_all(filter_group.to_ascii_lowercase().as_str(), "").to_string();

					new_aliases.push(aliases::Alias { alias, form, filter_group, });
				}
			}

			aliases::set_aliases(new_aliases);
			if alias_write_error.is_none() {
				match aliases::save_aliases() {
					Err(e) => {
						println!("x Error saving aliases");
						alias_write_error = Some(e.to_string())
					},
					_ => {},
				}
			}
		} else {
			aliases::load_aliases();
		}
		let aliases = aliases::get_aliases();
		let tpl = UiTemplate { aliases, alias_write_error };
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

fn named_spawns_response_callback(query: WebhookNamedEventQuery, spawn_file: &str, skyrim_path: String) -> reply::WithStatus<reply::Html<String>> {
	println!("> Received help request {}", json!(query));

	let form = formid_or_alias(query.form, query.alias_group);
	if form.is_empty() {
		println!("x Invalid form");
		return reply::with_status(reply::html("invalid form".to_string()), warp::http::StatusCode::BAD_REQUEST);
	}
	
	let path_buf = Path::new(&skyrim_path).join(spawn_file);
	let path = path_buf.as_path();
	let mut events = read_file_with_retry::<Vec<WebhookNamedEventQuery>>(path).unwrap_or_default();
	let new_event = WebhookNamedEventQuery {
		form,
		alias_group: String::new(),
		name: query.name
	};

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
}


pub async fn start_webinterface(port: u16, skyrim_path: String) {
	let skyrim_path_for_index = skyrim_path.clone();
	let index = warp::path::end()
		.and(warp::query::<WebhookEventQuery>())
		.and(warp::any().map(move || skyrim_path_for_index.clone()))
		.and(warp::body::form())
		.map(default_response_callback);

	let skyrim_path_for_help = skyrim_path.clone();
	let help = warp::path("help")
		.and(warp::query::<WebhookNamedEventQuery>())
		.and(warp::any().map(move || "spawns.ptw"))
		.and(warp::any().map(move || skyrim_path_for_help.clone()))
		.map(named_spawns_response_callback);

	let skyrim_path_for_enemy = skyrim_path.clone();
	let enemy = warp::path("enemy")
		.and(warp::query::<WebhookNamedEventQuery>())
		.and(warp::any().map(move || "enemies.ptw"))
		.and(warp::any().map(move || skyrim_path_for_enemy.clone()))
		.map(named_spawns_response_callback);

	let mut index_lock = FORM_INDEX.lock().unwrap();
	*index_lock = Some(formsearch::build_index());
	drop(index_lock);

	let search = warp::path("search")
		.and(warp::query::<SearchQuery>())
		.map(search_response_callback);

	let routes = index.or(search).or(help).or(enemy);

	println!("= Starting server on port {}", port);
	warp::serve(routes).run(([127, 0, 0, 1], port)).await;
}