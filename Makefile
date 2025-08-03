# Default installation prefix
PREFIX ?= /usr/local

# Installation directories
BINDIR := $(PREFIX)/bin
MANDIR := $(PREFIX)/share/man

# --- Main Targets ---

.PHONY: all
all: build

.PHONY: build-no-man
build-no-man:
	@echo "Building release binary without man pages..."
	@cargo build --release --no-default-features

.PHONY: build
build: clean
	@echo "Building release binary..."
	@cargo build --release

.PHONY: test
test:
	@echo "Testing..."
	@cargo test --no-default-features

.PHONY: install
install:
	@echo "Installing binary to $(BINDIR)..."
	@mkdir -p "$(BINDIR)"
	@install -m 755 target/release/hyprland-minimizer "$(BINDIR)/"

	@echo "Installing man pages to $(MANDIR)..."
	@mkdir -p "$(MANDIR)/man1"
	@mkdir -p "$(MANDIR)/man5"
	@install -m 644 target/release/build/*/out/hyprland-minimizer.1 "$(MANDIR)/man1/"
	@install -m 644 target/release/build/*/out/hyprland-minimizer.5 "$(MANDIR)/man5/"
	@echo "Installation complete."

.PHONY: uninstall
uninstall:
	@echo "Uninstalling binary..."
	@rm -f "$(BINDIR)/hyprland-minimizer"

	@echo "Uninstalling man pages..."
	@rm -f "$(MANDIR)/man1/hyprland-minimizer.1"
	@rm -f "$(MANDIR)/man5/hyprland-minimizer.5"
	@echo "Uninstallation complete."

.PHONY: clean
clean:
	@echo "Cleaning build artifacts..."
	@cargo clean

.PHONY: view-man1
view-man1: build
	@echo "Displaying generated man page 1..."
	@# The wildcard (*) handles the changing hash in the build directory.
	@man ./target/release/build/hyprland-minimizer-*/out/hyprland-minimizer.1

.PHONY: view-man5
view-man5: build
	@echo "Displaying generated man page 5..."
	@# The wildcard (*) handles the changing hash in the build directory.
	@man ./target/release/build/hyprland-minimizer-*/out/hyprland-minimizer.5
