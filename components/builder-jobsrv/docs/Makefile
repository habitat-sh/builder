DOT = dot




%.svg : %.dot
	$(DOT) -Tsvg $< -o $@

%.png : %.dot
	$(DOT) -Tpng $< -o $@
