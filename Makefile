
server:
	cd www; \
	npm run start

dist: release
	cd www; \
	npm run build;

release: esbuild
	wasm-pack build

dev: esbuild
	wasm-pack build --dev

esbuild:
	cd src/js; \
	npm run esbuild
