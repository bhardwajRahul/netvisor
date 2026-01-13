#!/usr/bin/env node

/**
 * Merges generated error messages into the main en.json translation file.
 *
 * This script:
 * 1. Reads the generated error-messages.json (from Rust ErrorCode enum)
 * 2. Reads the existing messages/en.json
 * 3. Merges error messages (errors_* keys) into en.json
 * 4. Preserves manual translations and other keys
 * 5. Writes the merged result back to en.json
 *
 * Run via: make generate-error-codes
 */

import { readFileSync, writeFileSync, existsSync } from 'fs';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const UI_DIR = join(__dirname, '..');

const GENERATED_FILE = join(UI_DIR, 'src/lib/generated/error-messages.json');
const MESSAGES_FILE = join(UI_DIR, 'messages/en.json');

function main() {
	// Check if generated file exists
	if (!existsSync(GENERATED_FILE)) {
		console.error(`Error: Generated file not found: ${GENERATED_FILE}`);
		console.error('Run `cargo run --bin generate-error-codes` first');
		process.exit(1);
	}

	// Read generated error messages
	const errorMessages = JSON.parse(readFileSync(GENERATED_FILE, 'utf-8'));
	console.log(`Read ${Object.keys(errorMessages).length} error messages from generated file`);

	// Read existing en.json (or start fresh)
	let existingMessages = {};
	if (existsSync(MESSAGES_FILE)) {
		existingMessages = JSON.parse(readFileSync(MESSAGES_FILE, 'utf-8'));
		console.log(`Read ${Object.keys(existingMessages).length} existing messages from en.json`);
	}

	// Remove old error messages (they'll be replaced with fresh ones)
	const manualMessages = {};
	let removedCount = 0;
	for (const [key, value] of Object.entries(existingMessages)) {
		if (key.startsWith('errors_')) {
			removedCount++;
		} else {
			manualMessages[key] = value;
		}
	}
	if (removedCount > 0) {
		console.log(`Removed ${removedCount} old error messages`);
	}

	// Merge: manual messages + new error messages
	const merged = {
		...manualMessages,
		...errorMessages
	};

	// Sort keys for consistent output
	const sorted = {};
	for (const key of Object.keys(merged).sort()) {
		sorted[key] = merged[key];
	}

	// Write back to en.json
	writeFileSync(MESSAGES_FILE, JSON.stringify(sorted, null, '\t') + '\n');
	console.log(`Wrote ${Object.keys(sorted).length} total messages to en.json`);
	console.log(`  - ${Object.keys(manualMessages).length} manual messages`);
	console.log(`  - ${Object.keys(errorMessages).length} error messages`);
}

main();
