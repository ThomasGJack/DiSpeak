import type { Config } from 'tailwindcss'

export default {
	content: [
		'./index.html',
		'./src/**/*.{ts,tsx}',
		'./src/components/**/*.{ts,tsx}',
		'./src/pages/**/*.{ts,tsx}',
	],
	theme: {
		extend: {},
	},
	plugins: [],
} satisfies Config


