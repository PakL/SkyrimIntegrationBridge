interface FormIndex {
	plugin: string;
	row: number;
}

const forms_index = new Map<string, FormIndex>();

export function buildIndex() {
	for(const file of Deno.readDirSync("./forms")) {
		if(file.isFile && file.name.toLocaleLowerCase().endsWith(".csv")) {
			const data = Deno.readTextFileSync("./forms/" + file.name);
			const lines = data.split("\n");

			let row = 0;
			for(const line of lines) {
				const columns = line.trim().split(";");
				if(columns.length < 5) continue;
				forms_index.set(`${columns[0].replace(/_/g,'')}${columns[2]}_${columns[3].replace(/[^a-z0-9]/i,"")}_${columns[4].replace(/[^a-z0-9]/i,"")}`.toLocaleLowerCase(), { plugin: columns[0], row });
				row++;
			}
		}
	}
}

export function findForms(query: string, filter_white: string[] = [], filter_black: string[] = []): string[][] {
	const keywords = query.toLocaleLowerCase().split(" ");
	for(const i in keywords) {
		keywords[i] = keywords[i].replace(/[^a-z0-9]/i,"");
	}
	for(const i in filter_white) {
		filter_white[i] = filter_white[i].toLocaleLowerCase();
	}
	for(const i in filter_black) {
		filter_black[i] = filter_black[i].toLocaleLowerCase();
	}

	const results: Map<string, FormIndex[]> = new Map<string, FormIndex[]>();

	for(const key of forms_index.keys()) {
		const key_parts = key.split("_");

		let includes = false;
		for(let i = 1; i < key_parts.length; i++) {
			let missing = false;
			for(const word of keywords) {
				if(key_parts[i].indexOf(word) < 0) {
					missing = true;
					break;
				}
			}
			if(!missing) includes = true;
		}

		if(!includes) continue;

		const index = forms_index.get(key);
		if(index === undefined) continue;

		const result = results.get(index.plugin)??[];
		result.push(index)
		results.set(index.plugin, result);
	}

	const final_results: string[][] = [];
	for(const plugin of results.keys()) {
		const data = Deno.readTextFileSync("./forms/" + plugin + ".csv");
		const lines = data.split("\n");
		const result = results.get(plugin);
		if(result === undefined) continue;
		for(const index of result) {
			if(lines.length <= index.row) continue;
			const line = lines[index.row];
			const columns = line.trim().split(";");
			if(filter_white.length > 0 && !filter_white.includes(columns[1].toLocaleLowerCase())) continue;
			if(filter_black.length > 0 && filter_black.includes(columns[1].toLocaleLowerCase())) continue;
			final_results.push(columns);
		}
	}

	return final_results;

}

function escapeHTML(text: string): string {
	return text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

export function generateHTML(forms_output: string[][]): string {
	let html = "";

	for(const form of forms_output) {
		if(form[4].length == 0) {
			form[4] = form[3];
		}
		html += `<div class="form"><button type="button" class="form-send" title="Send to Skyrim" onClick="webhookDialog('${form[1]}', '${form[0].replace(/'/g,"\\'")}|${form[2]}', '${form[4].replace(/'/g,"\\'")}')">🐲</button>`;
		if(form[4] !== form[3]) {
			html += `<div class="form-name">${escapeHTML(form[4])} <small class="muted">- ${escapeHTML(form[3])}</small></div>`;
		} else {
			html += `<div class="form-name">${escapeHTML(form[4])}</div>`;
		}
		
		html += `<div class="form-id">(${form[1]}) ${escapeHTML(form[0])}|${form[2]}</div>`;
		let lvl = [];
		switch(form[1]) {
			case "NPC_":
				html += `<div class="form-details">Race: ${form[5]}, Level: `;
				lvl = form[6].split(",");
				if(lvl.length == 1) {
					html += lvl[0];
				} else if(lvl.length == 3) {
					html += "PC&times;" + lvl[0] + ", Min. Lvl.: " + lvl[1] + ", Max. Lvl.: " + lvl[2];
				}
				html += `, Highest Attributes: ${escapeHTML(form[7])}</div>`;
				break;
			case "ALCH":
				html += `<div class="form-details">Weight: ${form[5]}, Value: ${form[6]}` + (form[7].length > 0 ? `, Effect: ${escapeHTML(form[7])}` : "") + "</div>";
				break;
			case "AMMO":
				html += `<div class="form-details">Damage: ${form[5]}, Value: ${form[6]}</div>`;
				break;
			case "ARMO":
				html += `<div class="form-details">Armor Type: ${escapeHTML(form[5])}, Armor Rating: ${form[6]}, Value: ${form[7]}, Weight: ${form[8]}</div>`;
				break;
			case "WEAP":
				html += `<div class="form-details">${escapeHTML(form[5])}, Damage: ${form[6]}, Value: ${form[7]}, Weight: ${form[8]}` + (form[9].length > 0 ? `, Effect: ${escapeHTML(form[9])}` : "") + "</div>";
				break;
			case "INGR":
				html += `<div class="form-details">Value: ${form[5]}, Weight: ${form[6]}</div>`;
				break;
			case "MISC":
				html += `<div class="form-details">Value: ${form[5]}, Weight: ${form[6]}</div>`;
				break;
			case "SPEL":
				html += `<div class="form-details">${escapeHTML(form[5])}, Base Cost: ${form[6]}` + (form[7].length > 0 ? `, Effect: ${escapeHTML(form[7])}` : "") + "</div>";
				break;
			case "SCRL":
				html += `<div class="form-details">` + (form[5].length > 0 ? `Effect: ${escapeHTML(form[5])}` : "") + "</div>";
				break;
		}
		html += "</div>";
	}

	return html;
}