# CHINJU Framework Paper - Build Configuration

MAIN = chinju_framework
LATEX = pdflatex
BIBTEX = bibtex

.PHONY: all clean watch

all: $(MAIN).pdf

$(MAIN).pdf: $(MAIN).tex
	$(LATEX) $(MAIN)
	$(LATEX) $(MAIN)

clean:
	rm -f $(MAIN).aux $(MAIN).log $(MAIN).out $(MAIN).toc
	rm -f $(MAIN).bbl $(MAIN).blg
	rm -f $(MAIN).pdf

watch:
	fswatch -o $(MAIN).tex | xargs -n1 -I{} make all

# For arXiv submission
arxiv: $(MAIN).pdf
	mkdir -p arxiv_submission
	cp $(MAIN).tex arxiv_submission/
	cp $(MAIN).bbl arxiv_submission/ 2>/dev/null || true
	cd arxiv_submission && tar -czvf ../chinju_arxiv.tar.gz *
	@echo "Created chinju_arxiv.tar.gz for arXiv submission"
