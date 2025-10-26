.PHONY: setup
setup:
	@if [ ! -d certs ]; then \
		mkdir certs; \
	fi; \
	if [ ! -d rcgen ]; then \
		git clone https://github.com/rustls/rcgen; \
	fi; \
	cd rcgen && \
	cargo run -- -o ../certs && \
	cargo install iroh-relay --features=server && \
	cd .. && \
	rm -rf rcgen

.PHONY: start-relay
start-relay:
	iroh-relay --config-path relay.conf --dev