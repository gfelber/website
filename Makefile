
server:
	cd www; \
	npm run start

publish: dist
	cd www/dist; \
	git add -A; \
	git commit -m "gfelber/website@`git --git-dir ../../.git log --format="%H" -n 1`"; \
	git push

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
