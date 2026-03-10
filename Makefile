BIN = pyrite
LINK = paru
PREFIX ?= /usr/local
BIN_DIR = $(DESTDIR)$(PREFIX)/bin

all:
	cargo build --release

install: all
	install -Dm755 target/release/$(BIN) $(BIN_DIR)/$(BIN)
	ln -sf $(BIN) $(BIN_DIR)/$(LINK)

uninstall:
	rm -f $(BIN_DIR)/$(BIN) $(BIN_DIR)/$(LINK)

clean:
	cargo clean
