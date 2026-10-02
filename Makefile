
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

release: esbuild prep
	wasm-pack build

dev: esbuild prep
	wasm-pack build --dev

esbuild:
	cd src/js; \
	npm run esbuild

SOURCES := $(filter-out latest.md,$(patsubst content/%,%,$(shell find content -type f)))
LATEST ?= $(shell readlink root/latest.md)

prep: $(addprefix root/,$(SOURCES)) root/latest.md

# teeo is just `tee output.log`, so render in a private dir (safe with make -j)
root/%: content/%
	@echo "prep $*"
	@mkdir -p $(@D)
	@tmp=$$(mktemp -d) && cd $$tmp && \
	PAGER=teeo script -efqO /dev/null -c "glow '$(CURDIR)/$<' -s dark -w 60 -p" >/dev/null && \
	sed -i -E 's/\x1B\[48;5;203m//g' output.log && \
	mv output.log '$(CURDIR)/$@' && rm -rf $$tmp

root/latest.md: root/$(LATEST)
	@ln -sfn $(LATEST) $@

.PHONY: prep root/latest.md

clean:
	cargo clean
	rm -rf dirs
