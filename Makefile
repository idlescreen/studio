OOHAT ?= /Projects/openOODA-agents/oohat/lint-oo-size
# The compiler follows OODA_HOME so a CI runner that stages the toolchain
# under /tmp resolves the same path the developer checkout does.
OODA_COMPILER ?= $(shell command -v oodac 2>/dev/null || if [ -x "$(OODA_HOME)/oodac/bin/oodac" ]; then echo "$(OODA_HOME)/oodac/bin/oodac"; elif [ -x "$(OODA_HOME)/bin/oodac" ]; then echo "$(OODA_HOME)/bin/oodac"; else echo "$(OODA_HOME)/oodac/bin/oodac"; fi)
# The oodar runtime ships in two layouts: a checkout tree (oodar/core/lib,
# oodar/scripts/lib) or an install tree (bin/liboodar.a). Point the compiler
# at whichever one actually holds the archive.
OODA_OODAR_ROOT ?= $(shell if [ -f "$(OODA_HOME)/oodar/core/lib/liboodar-core.a" ] || [ -f "$(OODA_HOME)/oodar/scripts/lib/liboodar-core.a" ] || [ -f "$(OODA_HOME)/oodar/lib/liboodar-core.a" ] || [ -f "$(OODA_HOME)/oodar/liboodar-core.a" ]; then echo "$(OODA_HOME)/oodar"; elif [ -f "$(OODA_HOME)/bin/liboodar.a" ] || [ -f "$(OODA_HOME)/bin/liboodar-core.a" ]; then echo "$(OODA_HOME)/bin"; else echo "$(OODA_HOME)/oodar"; fi)
export OODA_OODAR_ROOT
# The compiler resolves oostd/... imports by walking up from the source tree,
# so the std repo must be a directory named `oostd` in an ancestor of this
# project. It also has to sit inside the compiler's FS read scope.
OODA_HOME ?= $(shell if [ -x "$$HOME/.openooda/bin/oodac" ]; then echo "$$HOME/.openooda"; elif [ -d "$$HOME/Projects/openOODA-core/oodac" ]; then echo "$$HOME/Projects/openOODA-core"; elif [ -d /Projects/openOODA-core/oodac ]; then echo /Projects/openOODA-core; else echo "$(abspath $(CURDIR)/../../openOODA-core)"; fi)
OODA_STD_DIR ?= $(shell d=$(CURDIR); r=""; while [ "$$d" != "/" ]; do if [ -d "$$d/oostd" ]; then r="$$d/oostd"; break; fi; d=$$(dirname $$d); done; if [ -z "$$r" ] && [ -d "$(OODA_HOME)/oostd" ]; then r="$(OODA_HOME)/oostd"; fi; echo "$$r")
OODA_READDIR ?= $(CURDIR):$(OODA_HOME)$(if $(OODA_STD_DIR),:$(OODA_STD_DIR))
OODA_WRITEDIR ?= $(CURDIR):/tmp
OO_LIST_AMBIENT_QUOTA ?= 8589934592
BIN := bin/idle-studio
RENDER_BIN := bin/render
TESTBIN := bin/test_studio
SRC := main.oo anchor.oo $(wildcard core/*.oo raster/*.oo ascii/*.oo ascii/*/*.oo ascii/*/*/*.oo studio/*.oo)
ENV := OO_LIST_AMBIENT_QUOTA=$(OO_LIST_AMBIENT_QUOTA) OODA_FS_READDIR=$(OODA_READDIR) OODA_FS_WRITEDIR=$(OODA_WRITEDIR) OODA_COMPILER=$(OODA_COMPILER) OODAC_BIN=$(OODA_COMPILER) OODA_STD_DIR=$(OODA_STD_DIR)

.PHONY: all build verify test det clean check std-check line-cap file-law academy density vocab shape

all: build verify test det

build: $(BIN)
	@cp -f $(BIN) $(RENDER_BIN)
	@chmod +x $(BIN) $(RENDER_BIN)

$(BIN): $(SRC)
	@mkdir -p bin .ooda-cache
	@$(MAKE) --no-print-directory std-check
	@$(ENV) $(OODA_COMPILER) build main.oo -o $(BIN)

std-check:
	@test -n "$(OODA_STD_DIR)" || { echo "FAIL: no std repo named oostd in any ancestor of $(CURDIR)"; \
		echo "      clone openOODA-core/std as \`oostd\` beside this project"; exit 1; }
	@echo "PASS: std repo at $(OODA_STD_DIR)"
	@if [ ! -f "$(OODA_OODAR_ROOT)/liboodar.a" ] && [ ! -f "$(OODA_OODAR_ROOT)/liboodar-core.a" ] \
		&& [ ! -f "$(OODA_OODAR_ROOT)/core/lib/liboodar-core.a" ] && [ ! -f "$(OODA_OODAR_ROOT)/scripts/lib/liboodar-core.a" ]; then \
		echo "FAIL: no oodar runtime at $(OODA_OODAR_ROOT) — install the openOODA toolchain (https://openooda.org/install.sh)"; \
		exit 1; \
	fi

# The compiler's own typecheck over every page. The studio tree had no such
# gate, so unused imports and broken pages stayed invisible until a build.
check:
	@mkdir -p .ooda-cache
	@for f in $$(find . -name "*.oo" -not -path "./.git/*" -not -path "./bin/*" -not -path "./dist/*" -not -path "./.ooda-cache/*" -not -path "./tests/oo/*"); do \
		$(ENV) $(OODA_COMPILER) check "$$f" > .ooda-cache/check_out.txt 2>&1 || { echo "VIOLATION: oodac check fails on $$f"; echo "--- compiler output ---"; cat .ooda-cache/check_out.txt; exit 1; } \
	done; \
	echo "PASS: oodac check holds on all .oo files"

verify: line-cap file-law academy density vocab shape check
	@if [ -x "$(OOHAT)" ]; then $(OOHAT) $(CURDIR); \
	else echo "SKIP: page-rule linter not found at $(OOHAT) — set OOHAT to run it"; fi

line-cap:
	@violations=0; \
	for f in $$(find . -name "*.oo" -o -name "*.oot" | grep -v "/.git/" | grep -v "/bin/" | grep -v "/dist/" | grep -v "/.ooda-cache/" | grep -v "/target/"); do \
		n=$$(wc -l < "$$f"); \
		if [ $$n -gt 256 ]; then \
			echo "VIOLATION: $$f = $$n lines (exceeds 256)"; violations=$$((violations+1)); \
		fi; \
		code=$$(grep -vE '^[[:space:]]*(//.*)?$$' "$$f" | grep -cvE '^[[:space:]]*import[[:space:]]+"'); \
		if [ "$$code" = "0" ]; then continue; fi; \
		if [ $$n -lt 16 ]; then \
			echo "VIOLATION: $$f = $$n lines (under 16-line floor, not a shim)"; violations=$$((violations+1)); \
		fi; \
	done; \
	if [ $$violations -gt 0 ]; then echo "FAIL: $$violations files violate the Page Rule"; exit 1; fi; \
	echo "PASS: Page Rule sizing (16-256 lines, shims exempt from floor) holds"

file-law:
	@forbidden="py rs js ts rb pl json yaml toml"; \
	violations=0; \
	for ext in $$forbidden; do \
		found=$$(find . -name "*.$$ext" -not -path "./.git/*" -not -path "./.github/*" -not -path "./bin/*" -not -path "./dist/*" -not -path "./.ooda-cache/*" -not -path "./target/*" 2>/dev/null | head -3); \
		if [ -n "$$found" ]; then \
			echo "VIOLATION: .$$ext forbidden:"; echo "$$found"; violations=$$((violations+1)); \
		fi; \
	done; \
	yaml=$$(find . \( -name "*.yml" -o -name "*.yaml" \) -not -path "./.git/*" -not -path "./.github/workflows/*" -not -path "./.github/dependabot.yml" 2>/dev/null | head -3); \
	if [ -n "$$yaml" ]; then echo "VIOLATION: yaml outside .github"; echo "$$yaml"; violations=$$((violations+1)); fi; \
	for f in $$(find . -name "*.sh" -not -path "./.git/*" -not -path "./bin/*" -not -path "./dist/*" -not -path "./.ooda-cache/*" 2>/dev/null); do \
		b=$$(basename "$$f"); \
		if [ "$$b" != "install.sh" ] && [ "$$b" != "uninstall.sh" ]; then echo "VIOLATION: .sh forbidden outside install.sh and uninstall.sh: $$f"; violations=$$((violations+1)); fi; \
	done; \
	for f in $$(find . -name "*.md" -not -path "./.git/*" -not -path "./.github/*" -not -path "./bin/*" -not -path "./dist/*" -not -path "./.ooda-cache/*" 2>/dev/null); do \
		b=$$(basename "$$f"); \
		if [ "$$b" != "README.md" ] && [ "$$b" != "AGENTS.md" ]; then echo "VIOLATION: .md forbidden outside README.md and AGENTS.md: $$f"; violations=$$((violations+1)); fi; \
	done; \
	for nm in util utils helper helpers common misc shared base; do \
		found=$$(find . -name "$$nm.oo" -not -path "./.git/*" -not -path "./bin/*" -not -path "./dist/*" 2>/dev/null | head -3); \
		if [ -n "$$found" ]; then echo "VIOLATION: banned page name $$nm.oo:"; echo "$$found"; violations=$$((violations+1)); fi; \
	done; \
	if [ $$violations -gt 0 ]; then echo "FAIL: file-law violations"; exit 1; fi; \
	echo "PASS: file law holds"

academy:
	@failures=0; \
	for f in $$(find . -name "*.oo" -not -path "./.git/*" -not -path "./bin/*" -not -path "./dist/*" -not -path "./.ooda-cache/*"); do \
		header=$$(head -16 "$$f"); \
		missing=""; \
		echo "$$header" | grep -q "^// # "        || missing="$$missing title"; \
		echo "$$header" | grep -q "^// Logline:"  || missing="$$missing logline"; \
		echo "$$header" | grep -q "^// Setup:"    || missing="$$missing setup"; \
		echo "$$header" | grep -q "^// Beats:"    || missing="$$missing beats"; \
		if [ -n "$$missing" ]; then \
			echo "FAIL: $$f missing Academy element(s):$$missing"; failures=$$((failures+1)); \
		fi; \
	done; \
	if [ $$failures -gt 0 ]; then echo "FAIL: $$failures academy header violations"; exit 1; fi; \
	echo "PASS: academy headers hold"

density:
	@violations=0; \
	for d in $$(find . -type d -not -path "./.git*" -not -path "./bin*" -not -path "./dist*" -not -path "./.ooda-cache*" -not -path "./.github*" -not -path "./target*"); do \
		n=$$(ls "$$d"/*.oo "$$d"/*.oot 2>/dev/null | grep -v '\*' | wc -l); \
		if [ $$n -gt 8 ]; then \
			echo "VIOLATION: $$d holds $$n pages (exceeds 8)"; violations=$$((violations+1)); \
		fi; \
	done; \
	if [ $$violations -gt 0 ]; then echo "FAIL: $$violations directories exceed density bound"; exit 1; fi; \
	echo "PASS: directory density (<= 8 pages per directory) holds"

vocab:
	@if grep -riE "sovereign|Pillar [0-9]|Stage [0-9]|bootstrap|moonshot|dogfight" --include="*.oo" --include="*.oot" --include="*.md" --include="*.pkg" --include="*.yml" . ; then \
		echo "FAIL: banned vocabulary found"; exit 1; \
	fi; \
	echo "PASS: vocabulary check holds"

shape:
	@violations=0; \
	for f in $$(find . -name "*.oo" -not -path "./.git/*" -not -path "./bin/*" -not -path "./dist/*" -not -path "./.ooda-cache/*"); do \
		fns=$$(grep -cE '^[[:space:]]*(pub[[:space:]]+)?fn ' "$$f" || true); \
		if [ "$$fns" -gt 8 ]; then \
			echo "VIOLATION: $$f has $$fns functions (exceeds 8)"; violations=$$((violations+1)); \
		fi; \
		long=$$(awk '/^[[:space:]]*(pub[[:space:]]+)?fn /{s=NR;d=0;b=0} s>0{for(i=1;i<=length($$0);i++){c=substr($$0,i,1);if(c=="{"){d++;b=1}else if(c=="}")d--} if(b&&d==0){l=NR-s+1;if(l>64)print s":"l;s=0}}' "$$f"); \
		if [ -n "$$long" ]; then \
			echo "VIOLATION: $$f has function(s) > 64 lines ($$long)"; violations=$$((violations+1)); \
		fi; \
	done; \
	if [ $$violations -gt 0 ]; then echo "FAIL: shape law violations"; exit 1; fi; \
	echo "PASS: function count (<= 8 per page) and length (<= 64 lines) hold"

test: build
	@rm -rf .ooda-cache
	@mkdir -p .ooda-cache
	@$(ENV) $(OODA_COMPILER) build tests/test_main.oo -o $(TESTBIN)
	@$(ENV) ./$(TESTBIN)
	@rm -f $(TESTBIN)
	@$(ENV) ./bin/idle-studio --self-test
	@$(ENV) ./bin/render --persona render -e beams --duration 1s --dry-run -o /tmp/test-beams.mkv | grep -q "dry-run: effect=beams" && echo "PASS: render dry-run"
	@$(ENV) ./bin/render -e beams --duration 1s --dry-run -o /tmp/test-beams-alias.mkv | grep -q "dry-run: effect=beams" && echo "PASS: render alias dispatch"
	@$(ENV) ./bin/render --persona render -e plasma --duration 1s --fps 1 --format raw -o /tmp/test-real.argb && test "$$(wc -c < /tmp/test-real.argb)" -eq 983040 && echo "PASS: render real encode (byte-exact raw)"
	@$(ENV) ./bin/idle-studio --queue .ooda-cache/q.json enqueue -e aurora --duration 2s --dry-run -o /tmp/test-aurora.mkv | grep -q "enqueued job-1" && echo "PASS: studio enqueue"
	@$(ENV) ./bin/idle-studio --queue .ooda-cache/q.json list | grep -q "job-1 pending aurora" && echo "PASS: studio list"
	@$(ENV) ./bin/idle-studio --queue .ooda-cache/q.json run --all | grep -q "done job-1" && echo "PASS: studio run"
	@rm -rf .ooda-cache
	@echo "PASS: all idle-studio & render tests"

det: build
	@mkdir -p .ooda-cache
	@$(ENV) $(OODA_COMPILER) build main.oo -o .ooda-cache/det_a > /dev/null
	@$(ENV) $(OODA_COMPILER) build main.oo -o .ooda-cache/det_b > /dev/null
	@cmp -s .ooda-cache/det_a .ooda-cache/det_b && rm -rf .ooda-cache && echo "PASS: deterministic build (byte-identical)"

clean:
	@rm -rf .ooda-cache $(TESTBIN)
