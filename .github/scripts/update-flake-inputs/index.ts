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

const main = async () => {
	const flake: FlakeMetadata = await $`nix flake metadata --json`.json();

	const inputNames = Object.keys(flake.locks.nodes);

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

	for (const input of inputsToUpdate) {
		console.log(`$ nix flake lock --update-input ${input}`.yellow);
		await $`nix flake lock --update-input ${input}`;
	}
};

await main();
