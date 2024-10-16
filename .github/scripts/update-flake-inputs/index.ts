#!/usr/bin/env bun

import { $ } from "bun";
import "colors";

interface FlakeMetadata {
	description: string;
	lastModified: number;
	locked: {
		lastModified: number;
		narHash: string;
		ref: string;
		rev: string;
		revCount: number;
		type: "git";
		url: string;
	};
	locks: {
		nodes: {
			[key: string]: {
				inputs: {
					[key: string]: string[] | string;
				};
				locked: {
					lastModified: number;
					narHash: string;
					owner: string;
					repo: string;
					rev: string;
					type: "github";
				};
				original: {
					owner: string;
					repo: string;
					type: "github";
				};
			};
		};
		root: "root";
		version: number;
	};
	original: {
		type: "git";
		url: string;
	};
	originalUrl: string;
	path: string;
	resolved: {
		type: "git";
		url: string;
	};
	resolvedUrl: string;
	revCount: number;
	revision: string;
	url: string;
}

class Flake {
	public meta: FlakeMetadata;

	constructor(meta: FlakeMetadata) {
		this.meta = meta;
	}

	static async getCurrentState(): Promise<FlakeMetadata> {
		return await $`nix flake metadata --json`.json();
	}

	static async init(): Promise<Flake> {
		const metadata = await Flake.getCurrentState();

		return new Flake(metadata);
	}

	async refresh(): Promise<void> {
		const meta = await Flake.getCurrentState();
		this.meta = meta;
	}
}

const main = async () => {
	const flake = await Flake.init();

	const inputNames = Object.keys(flake.meta.locks.nodes);

	const inputsToUpdate = [...new Set(process.argv.slice(2))];
	if (inputsToUpdate.length === 0) {
		console.log("no inputs to update".yellow);
		return;
	}

	const missingInputs = inputsToUpdate.filter(
		(inputName) => !inputNames.includes(inputName),
	);
	if (missingInputs.length > 0) {
		console.log(
			`input(s) not available in flake:\n - ${missingInputs.join("\n - ")}`.red,
		);
		process.exit(1);
	}

	const epochToHumanReadableDate = (epoch: number) =>
		new Date(epoch * 1000).toLocaleDateString("en-US", {
			hour: "2-digit",
			minute: "2-digit",
			year: "numeric",
			month: "long",
			day: "numeric",
		});

	for (const input of inputsToUpdate) {
		const since = epochToHumanReadableDate(
			flake.meta.locks.nodes[input].locked.lastModified,
		);

		console.log(`$ nix flake lock --update-input ${input}`.yellow);
		await $`nix flake lock --update-input ${input}`;

		await flake.refresh();
		const until = epochToHumanReadableDate(
			flake.meta.locks.nodes[input].locked.lastModified,
		);

		if (since === until) {
			console.log(
				`no new changes detected for input '${input}' since '${since}'`.magenta,
			);
			continue;
		}

		console.log(`new changes were detected since '${since}'`.blue);

		await $`git add flake.lock`;
		await $`git commit -m "flake-update(${input}): automated change (${since} -> ${until})"`;
	}
};

await main();
