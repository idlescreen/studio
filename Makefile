OODA_COMPILER ?= $(shell command -v oodac 2>/dev/null || echo /Projects/openOODA-core/oodac/bin/oodac)
BIN := bin/idle-studio
RENDER_BIN := bin/render
TESTBIN := bin/test_studio
SRC := main.oo anchor.oo $(wildcard core/*.oo raster/*.oo ascii/*.oo ascii/*/*.oo ascii/*/*/*.oo studio/*.oo)

.PHONY: all build verify test det clean

all: build verify test det

build: $(BIN)
	@cp -f $(BIN) $(RENDER_BIN)
	@chmod +x $(BIN) $(RENDER_BIN)

$(BIN): $(SRC)
	@mkdir -p bin .ooda-cache
	@$(OODA_COMPILER) build main.oo -o $(BIN)

verify:
	@/Projects/openOODA-agents/oohat/lint-oo-size $(CURDIR)

test: build
	@mkdir -p .ooda-cache
	@$(OODA_COMPILER) build tests/test_main.oo -o $(TESTBIN)
	@./$(TESTBIN)
	@rm -f $(TESTBIN)
	@./bin/idle-studio --self-test
	@./bin/render -e beams --duration 1s --dry-run -o /tmp/test-beams.mkv | grep -q "dry-run: effect=beams" && echo "PASS: render dry-run"
	@./bin/idle-studio --queue .ooda-cache/q.json enqueue -e aurora --duration 2s --dry-run -o /tmp/test-aurora.mkv | grep -q "enqueued job-1" && echo "PASS: studio enqueue"
	@./bin/idle-studio --queue .ooda-cache/q.json list | grep -q "job-1 pending aurora" && echo "PASS: studio list"
	@./bin/idle-studio --queue .ooda-cache/q.json run --all | grep -q "done job-1" && echo "PASS: studio run"
	@rm -rf .ooda-cache
	@echo "PASS: all idle-studio & render tests"

det: build
	@mkdir -p .ooda-cache
	@$(OODA_COMPILER) build main.oo -o .ooda-cache/det_a > /dev/null
	@$(OODA_COMPILER) build main.oo -o .ooda-cache/det_b > /dev/null
	@cmp -s .ooda-cache/det_a .ooda-cache/det_b && rm -rf .ooda-cache && echo "PASS: deterministic build (byte-identical)"

clean:
	@rm -rf .ooda-cache $(TESTBIN)
