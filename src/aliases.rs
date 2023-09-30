use std::sync::Mutex;
use std::collections::HashMap;
use itertools::Itertools;
use std::fs::File;
use std::io::{ Read, Write };

use serde::{ Deserialize, Serialize };
use serde_json;

#[derive(Serialize, Deserialize, Clone)]
pub struct Alias {
	pub alias: String,
	pub form: String,
	pub filter_group: String,
}

static ALIASES: Mutex<Option<HashMap<String, Alias>>> = Mutex::new(None);

pub fn load_aliases() {
	println!("= Loading aliases file");
	
	let aliases_file_result = File::open("aliases.json");
	if aliases_file_result.is_err() {
		println!("x Could not open aliases file");
		println!("x {}", aliases_file_result.err().unwrap());
		return;
	}

	let mut aliases_file = aliases_file_result.unwrap();
	let mut json_buffer: String = String::new();
	aliases_file.read_to_string(&mut json_buffer).unwrap();

	match serde_json::from_str::<Vec<Alias>>(json_buffer.as_str()) {
		Ok(aliases) => {
			let mut lock = ALIASES.lock().unwrap();
			let mut map = HashMap::new();
			for alias in aliases {
				map.insert(alias.alias.clone(), alias);
			}
			*lock = Some(map);
		},
		Err(e) => {
			println!("x Invalid aliases file");
			println!("x {}", e);
		},
	}
}

pub fn get_alias(alias: &String) -> Option<Alias> {
	let lock = ALIASES.lock().unwrap();
	
	if let Some(map) = &*lock {
		if let Some(alias) = map.get(alias) {
			return Some(alias.clone());
		}
	}
	None
}

pub fn get_aliases() -> Vec<Alias> {
	let lock = ALIASES.lock().unwrap();
	
	let mut aliases = Vec::new();
	if let Some(map) = &*lock {
		for key in map.keys().sorted() {
			aliases.push(map[key].clone());
		}
	}
	aliases
}

pub fn set_aliases(aliases: Vec<Alias>) {
	let mut lock = ALIASES.lock().unwrap();
	let mut map = HashMap::new();
	for alias in aliases {
		map.insert(alias.alias.clone(), alias);
	}
	*lock = Some(map);
}

pub fn save_aliases() -> std::io::Result<()> {
	let lock = ALIASES.lock().unwrap();
	
	if let Some(map) = &*lock {
		println!("= Writing aliases file");

		let mut aliases = Vec::new();
		for key in map.keys().sorted() {
			aliases.push(map[key].clone());
		}
		let json = serde_json::to_string_pretty(&aliases).unwrap();
		let mut file = File::create("aliases.json")?;
		file.write_all(json.as_bytes()).unwrap();
	} else {
		println!("= No aliases data to write");
	}
	Ok(())
}