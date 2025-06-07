import { defineConfig } from 'tailwindcss';

export default defineConfig({
    content: ['./src/**/*.{astro,html,js,jsx,md,mdx,svelte,ts,tsx,vue}'],
    theme: {
        extend: {},
    },
    plugins: [
        require('@tailwindcss/typography'),
    ],
}); 