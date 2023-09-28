use std::collections::HashMap;
use std::vec;
use regex::Regex;

use std::{ fs, fs::File };
use std::io::Read;

#[derive(Clone)]
pub struct FormIndex {
	plugin: String,
	row: u32,
}

pub enum Form {
	NPC_ { plugin: String, form_type: String, formid: String, editorid: String, name: String, race: String, level: String, attributes: String, },
	ALCH { plugin: String, form_type: String, formid: String, editorid: String, name: String, weight: String, value: String, effect: String, },
	AMMO { plugin: String, form_type: String, formid: String, editorid: String, name: String, damage: String, value: String, },
	ARMO { plugin: String, form_type: String, formid: String, editorid: String, name: String, armor_type: String, rating: String, value: String, weight: String },
	WEAP { plugin: String, form_type: String, formid: String, editorid: String, name: String, skill: String, damage: String, value: String, weight: String, effect: String, },
	INGR { plugin: String, form_type: String, formid: String, editorid: String, name: String, value: String, weight: String, },
	MISC { plugin: String, form_type: String, formid: String, editorid: String, name: String, value: String, weight: String, },
	SPEL { plugin: String, form_type: String, formid: String, editorid: String, name: String, spell_type: String, cost: String, effect: String },
	SCRL { plugin: String, form_type: String, formid: String, editorid: String, name: String, effect: String, },
	Other{ plugin: String, form_type: String, formid: String, editorid: String, name: String, },
}

pub fn build_index() -> HashMap<String, FormIndex> {
	let mut result: HashMap<String, FormIndex> = HashMap::new();

	println!("= Building form index");
	let allow_ascii = Regex::new(r"[^a-zA-Z0-9_]").unwrap();
	if let Ok(dir) = fs::read_dir("forms") {
		for file in dir {
			let path = file.unwrap().path();
			if path.is_file() {
				if let Some(ext) = path.extension() {
					if ext == "csv" {
						let mut file = File::open(path.clone()).unwrap();
						let mut buf = String::new();
						file.read_to_string(&mut buf).unwrap();

						let filename = path.file_name().unwrap().to_str().unwrap().to_string();
						let plugin = filename[..filename.len() - 4].to_string();

						let lines = buf.lines();
						let mut i: u32 = 0;
						for line in lines {
							let split: Vec<&str> = line.split(';').collect();
							
							if split.len() < 10 {
								continue;
							}
							let editorid = allow_ascii.replace_all(split.get(3).unwrap(), "").to_string();
							let editorname = allow_ascii.replace_all(split.get(4).unwrap(), "").to_string();
							let key = format!("{}{}_{}_{}", split.get(0).unwrap().to_string().replace("_", ""), split.get(2).unwrap(), editorid, editorname).to_lowercase();

							result.insert(key, FormIndex { plugin: plugin.clone(), row: i });
							i += 1;
						}
					}
				}
			}
		}
	}


	result
}

pub fn find_forms(index: &HashMap<String, FormIndex>, query: &str, filter_white: Vec<&str>, filter_black: Vec<&str>) -> Vec<Form> {
	let query_lc = query.clone().to_lowercase();
	let query_parts: Vec<&str> = query_lc.split(' ').collect();

	let mut findings: HashMap<String, Vec<FormIndex>> = HashMap::new();

	for (k, v) in index.iter() {
		let key_parts: Vec<&str> = k.split('_').collect();
		let mut includes = false;
		for i in 1..key_parts.len() {
			let key_part = key_parts.get(i).unwrap();
			let mut missing = false;
			for part in &query_parts {
				if !key_part.contains(part) {
					missing = true;
					break;
				}
			}
			if !missing {
				includes = true;
				break;
			}
		}

		if !includes {
			continue;
		}

		findings.entry(v.plugin.clone()).and_modify(|f| f.push(v.clone())).or_insert(vec![v.clone()]);
	}

	let mut result: Vec<Form> = vec![];
	for (plugin, finds) in findings.iter() {
		let mut file = File::open(format!("forms/{}.csv", plugin)).unwrap();
		let mut buf = String::new();
		file.read_to_string(&mut buf).unwrap();

		let lines: Vec<&str> = buf.lines().collect();
		for find in finds {
			let line = lines.get(find.row as usize).unwrap();
			let split: Vec<String> = line.split(';').map(|s| s.to_string()).collect();
			if split.len() < 10 {
				continue;
			}

			if filter_white.len() > 0 && !filter_white.contains(&split.get(1).unwrap().as_str()) {
				continue;
			}
			if filter_black.contains(&split.get(1).unwrap().as_str()) {
				continue;
			}

			let plugin = split.get(0).unwrap().clone();
			let form_type = split.get(1).unwrap().clone();
			let formid = split.get(2).unwrap().clone();
			let editorid = split.get(3).unwrap().clone();
			let mut name = split.get(4).unwrap().clone();
			if name.is_empty() {
				name = editorid.clone();
			}

			result.push(match split.get(1).unwrap().clone().as_str() {
				"NPC_" => Form::NPC_ {
					plugin, form_type, formid, editorid, name,
					race: split.get(5).unwrap().clone(),
					level: split.get(6).unwrap().clone(),
					attributes: split.get(7).unwrap().clone(),
				},
				"ALCH" => Form::ALCH {
					plugin, form_type, formid, editorid, name,
					weight: split.get(5).unwrap().clone(),
					value: split.get(6).unwrap().clone(),
					effect: split.get(7).unwrap().clone(),
				},
				"AMMO" => Form::AMMO {
					plugin, form_type, formid, editorid, name,
					damage: split.get(5).unwrap().clone(),
					value: split.get(6).unwrap().clone(),
				},
				"ARMO" => Form::ARMO {
					plugin, form_type, formid, editorid, name,
					armor_type: split.get(5).unwrap().clone(),
					rating: split.get(6).unwrap().clone(),
					value: split.get(7).unwrap().clone(),
					weight: split.get(8).unwrap().clone(),
				},
				"WEAP" => Form::WEAP {
					plugin, form_type, formid, editorid, name,
					skill: split.get(5).unwrap().clone(),
					damage: split.get(6).unwrap().clone(),
					value: split.get(7).unwrap().clone(),
					weight: split.get(8).unwrap().clone(),
					effect: split.get(9).unwrap().clone(),
				},
				"INGR" => Form::INGR {
					plugin, form_type, formid, editorid, name,
					value: split.get(5).unwrap().clone(),
					weight: split.get(6).unwrap().clone(),
				},
				"MISC" => Form::MISC {
					plugin, form_type, formid, editorid, name,
					value: split.get(5).unwrap().clone(),
					weight: split.get(6).unwrap().clone(),
				},
				"SPEL" => Form::SPEL {
					plugin, form_type, formid, editorid, name,
					spell_type: split.get(5).unwrap().clone(),
					cost: split.get(6).unwrap().clone(),
					effect: split.get(7).unwrap().clone(),
				},
				_ => Form::Other { plugin, form_type, formid, editorid, name, }
			});
		}
	}

	result
}