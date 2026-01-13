import { sveltekit } from '@sveltejs/kit/vite';
import { paraglideVitePlugin } from '@inlang/paraglide-js';
import { defineConfig } from 'vite';
import pkg from './package.json';

export default defineConfig({
	plugins: [
		sveltekit(),
		paraglideVitePlugin({
			project: './project.inlang',
			outdir: './src/lib/paraglide'
		})
	],
	define: {
		__APP_VERSION__: JSON.stringify(pkg.version)
	},
	server: {
		host: '0.0.0.0',
		allowedHosts: ['scanopy-dev.local'],
		port: 5173,
		proxy: {
			'/api': {
				target: 'http://localhost:60072',
				changeOrigin: true
			}
		}
	},

	build: {
		outDir: 'build'
	}
});
