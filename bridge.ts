import { buildIndex, findForms, generateHTML } from "./ui.ts";

interface SkyrimIntegrationSpawn {
	name: string;
	form: string;
}

interface SkyrimIntegrationEvent {
	type: number;
	form: string;
	count: number;
}

function readPTW<T>(name: string, events: T[]): T[] {
	let not_read = true;
	let try_count = 0;
	while(not_read) {
		try {
			events = JSON.parse(Deno.readTextFileSync(config.skyrimpath + "/" + name));
			not_read = false;
		} catch {
			console.log(`= Unable to read ${name}`);
			try_count++;
			if(try_count > 3) {
				console.log(`= Giving up reading ${name} after 3 tries`);
				not_read = false;
			}
		}
	}
	return events;
}
function writePTW(name: string, events: SkyrimIntegrationSpawn[]|SkyrimIntegrationEvent[]): boolean {
	let not_written = true;
	let adding_failed = false;
	let try_count = 0;
	while(not_written) {
		try {
			Deno.writeTextFileSync(config.skyrimpath + "/" + name, JSON.stringify(events));
			not_written = false;
			console.log(`< Added event to ${name}`);
		} catch {
			console.log(`= Unable to write ${name}`);
			try_count++;
			if(try_count > 3) {
				console.log(`= Giving up writing ${name} after 3 tries`);
				not_written = false;
				adding_failed = true;
			}
		}
	}
	return !adding_failed;
}

let index_build = false;

async function handleHttp(conn: Deno.Conn) {
	for await (const e of Deno.serveHttp(conn)) {
		const url = new URL(e.request.url);

		console.log(`> ${url.pathname}${url.search}`);

		let filename = "events.ptw";
		let name = "";
		let type = -1;
		let form = "";
		let count = 1;
		let search = "";
		url.searchParams.forEach((value, key) => {
			switch(key) {
				case "name": name = value; break;
				case "query": search = value; break;
				case "type": type = parseInt(value); break;
				case "form": {
					const formsplits: string[] = value.split("|");
					if(formsplits.length == 1) formsplits.unshift("Skyrim.esm");
					if(formsplits.length == 2) formsplits.unshift("__formData");
					if(formsplits.length > 3 || !formsplits[2].match(/^(0x)?[0-F]{1,6}$/i)) {
						form = "";
					} else {
						if(!formsplits[2].startsWith("0x")) formsplits[2] = "0x" + formsplits[2];
						form = formsplits.join("|");
					}
				} break;
				case "count": count = parseInt(value); break;
			}
		});

		if(url.pathname.match(/^\/help/i)) {
			filename = "spawns.ptw";
		} else if(url.pathname.match(/^\/enemy/i)) {
			filename = "enemies.ptw";
		} else if(url.pathname.match(/^\/search/i)) {
			if(!index_build) {
				console.log("= Building index");
				buildIndex();
				index_build = true;
			}
			if(search.length > 0) {
				e.respondWith(new Response(generateHTML(findForms(search, [], ["ENCH", "MGEF"]))));
			} else {
				e.respondWith(new Response(null, { status: 204 }));
			}
			continue;
		} else if(type < 0) {
			try {
				e.respondWith(new Response(Deno.readTextFileSync("./ui.html"), { headers: { "Content-Type": "text/html" } }));
			} catch {
				e.respondWith(new Response("unable to load ui.html", { status: 404 }));
			}
			continue;
		}


		if(filename === "events.ptw" && type > -1 && type <= 5 && form.length > 0) {
			if(count < 1) count = 1;
			if(type == 0 && count > 128) count = 128;

			let events: SkyrimIntegrationEvent[] = [];
			events = readPTW(filename, events);
			events.push({ type, form, count });
			const adding_failed = !writePTW(filename, events);
			e.respondWith(new Response((adding_failed ? "could not create event" : "ok"), { status: adding_failed ? 500 : 200 }));
		} else if((filename === "spawns.ptw" || filename === "enemies.ptw") && name.length > 0 && form.length > 0) {
			let events: SkyrimIntegrationSpawn[] = [];
			events = readPTW(filename, events);
			events.push({ name, form });
			const adding_failed = !writePTW(filename, events);
			e.respondWith(new Response((adding_failed ? "could not create event" : "ok"), { status: adding_failed ? 500 : 200 }));
		} else {
			e.respondWith(new Response("not found", { status: 404 }));
		}

	}
}

console.log("= Loading config file");
const config = JSON.parse(Deno.readTextFileSync("config.json"));
if(typeof(config.port) !== "number" || typeof(config.skyrimpath) !== "string") {
	console.log("= Invalid config file");
	Deno.exit(1);
} else {
	try {
		Deno.statSync(config.skyrimpath + "/events.ptw");
	} catch {
		Deno.writeTextFileSync(config.skyrimpath + "/events.ptw", "[]");
	}
	try {
		Deno.statSync(config.skyrimpath + "/enemies.ptw");
	} catch {
		Deno.writeTextFileSync(config.skyrimpath + "/enemies.ptw", "[]");
	}
	try {
		Deno.statSync(config.skyrimpath + "/spawns.ptw");
	} catch {
		Deno.writeTextFileSync(config.skyrimpath + "/spawns.ptw", "[]");
	}
}

console.log("= Skyrim path: " + config.skyrimpath);

console.log("= Starting server on port " + config.port);
for await (const conn of Deno.listen({ port: config.port })) {
	await handleHttp(conn);
}