const esbuild = require('esbuild');
esbuild.build({
    entryPoints: ['import.js'],
    bundle: true,
    outfile: 'dist/package.js',
    format: 'esm',
    minify: true,
}).catch(() => process.exit(1));