.PHONY: build serve clean

build:
	cargo run

serve:
	npx http-server public -p 8080 --cors

clean:
	rm -rf target
