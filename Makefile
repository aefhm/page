.PHONY: build clean preview serve

build:
	cargo run

preview: build
	(sleep 1; open http://127.0.0.1:7778) &
	npx http-server public -a 127.0.0.1 -p 7778 --cors

serve: build
	npx http-server public -a 127.0.0.1 -p 7778 --cors 

clean:
	rm -rf public target
