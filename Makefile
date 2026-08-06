HOST ?= 127.0.0.1
PORT ?= 7778

.PHONY: build clean preview serve

build:
	cargo run

preview: build
	(sleep 1; open http://$(HOST):$(PORT)) &
	cd public && python3 -m http.server $(PORT) --bind $(HOST)

serve: build
	@watchexec \
	--watch src \
	--watch pages \
	--watch writings \
	--watch recipes \
	--watch static \
	--watch Cargo.toml \
	-- cargo run & \
	cd public && python3 -m http.server $(PORT) --bind $(HOST)

clean:
	rm -rf public target
