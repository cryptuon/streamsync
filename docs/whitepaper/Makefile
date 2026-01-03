# StreamSync Whitepaper Build System
# Usage: make [target]
# Targets: pdf, clean, watch, open

# Configuration
LATEX = pdflatex
BIBTEX = bibtex
MAIN = streamsync-whitepaper
OUTPUT_DIR = build

# Default target
.PHONY: all
all: pdf

# Create output directory
$(OUTPUT_DIR):
	mkdir -p $(OUTPUT_DIR)

# Build PDF with bibliography
.PHONY: pdf
pdf: $(OUTPUT_DIR)
	$(LATEX) -output-directory=$(OUTPUT_DIR) $(MAIN).tex
	cp references.bib $(OUTPUT_DIR)/
	cd $(OUTPUT_DIR) && $(BIBTEX) $(MAIN)
	$(LATEX) -output-directory=$(OUTPUT_DIR) $(MAIN).tex
	$(LATEX) -output-directory=$(OUTPUT_DIR) $(MAIN).tex
	@echo "PDF generated: $(OUTPUT_DIR)/$(MAIN).pdf"

# Build with bibliography (if references.bib exists)
.PHONY: full
full: $(OUTPUT_DIR)
	$(LATEX) -output-directory=$(OUTPUT_DIR) $(MAIN).tex
	@if [ -f references.bib ]; then \
		cd $(OUTPUT_DIR) && $(BIBTEX) $(MAIN); \
		cd .. && $(LATEX) -output-directory=$(OUTPUT_DIR) $(MAIN).tex; \
	fi
	$(LATEX) -output-directory=$(OUTPUT_DIR) $(MAIN).tex
	@echo "PDF generated: $(OUTPUT_DIR)/$(MAIN).pdf"

# Clean auxiliary files
.PHONY: clean
clean:
	rm -rf $(OUTPUT_DIR)
	rm -f *.aux *.log *.out *.toc *.bbl *.blg *.fdb_latexmk *.fls *.synctex.gz
	@echo "Cleaned build files"

# Clean and rebuild
.PHONY: rebuild
rebuild: clean pdf

# Watch for changes and rebuild (requires fswatch or inotifywait)
.PHONY: watch
watch:
	@echo "Watching for changes... (Ctrl+C to stop)"
	@while true; do \
		inotifywait -qe modify $(MAIN).tex 2>/dev/null || fswatch -1 $(MAIN).tex 2>/dev/null; \
		make pdf; \
	done

# Open PDF (Linux/macOS)
.PHONY: open
open: pdf
	@if [ "$$(uname)" = "Darwin" ]; then \
		open $(OUTPUT_DIR)/$(MAIN).pdf; \
	elif [ "$$(uname)" = "Linux" ]; then \
		xdg-open $(OUTPUT_DIR)/$(MAIN).pdf 2>/dev/null || evince $(OUTPUT_DIR)/$(MAIN).pdf; \
	fi

# Quick build (single pass, for drafts)
.PHONY: quick
quick: $(OUTPUT_DIR)
	$(LATEX) -output-directory=$(OUTPUT_DIR) $(MAIN).tex
	@echo "Quick build complete: $(OUTPUT_DIR)/$(MAIN).pdf"

# Help
.PHONY: help
help:
	@echo "StreamSync Whitepaper Build System"
	@echo ""
	@echo "Usage: make [target]"
	@echo ""
	@echo "Targets:"
	@echo "  pdf     - Build PDF (default)"
	@echo "  full    - Build with bibliography"
	@echo "  quick   - Single-pass build (faster, for drafts)"
	@echo "  clean   - Remove build files"
	@echo "  rebuild - Clean and rebuild"
	@echo "  watch   - Watch for changes and rebuild"
	@echo "  open    - Build and open PDF"
	@echo "  help    - Show this help"
