
PAGES = \
	built/TWiR_CotW_list.html

HAVE_NIX_SHELL = which nix-shell >/dev/null 2>&1

ASCIIDOCTOR_RENDER_HTML = \
	asciidoctor -b xhtml5 \
	-o "$@" "built/$*.adoc"

build:
	@nix-shell --run 'cargo build'

run:
	@if $(HAVE_NIX_SHELL); then \
	   nix-shell --run 'cargo run'; \
	 else \
	   cargo run; \
	 fi

clean:
	@nix-shell --run 'cargo clean'

fmt:
	@nix-shell --run 'cargo-fmt'

shell:
	@nix-shell

html: run $(PAGES)

view-html: run $(PAGES)
	@chromium-browser $(PAGES)

built/%.html: built/%.adoc
	@if $(HAVE_NIX_SHELL); then \
	   nix-shell --run '$(ASCIIDOCTOR_RENDER_HTML)'; \
	 else \
	   $(ASCIIDOCTOR_RENDER_HTML); \
	 fi
