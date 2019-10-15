INCLUDE:=-I$(shell pg_config --includedir)
LIBS:=-L$(shell pg_config --libdir) -lpgcommon -lpgport -lpthread -lpq
CFLAGS=-Wall -pedantic -Wextra -O3
.PHONY: clobber clean

all: pmetrics-logger pmetrics-measure

pmetrics-logger: pmetrics-logger.cpp pmetrics-common.o
	g++ $(INCLUDE) $(CFLAGS) pmetrics-logger.cpp -c
	g++ $(INCLUDE) pmetrics-logger.o pmetrics-common.o -o pmetrics-logger  $(LIBS)

pmetrics-measure: pmetrics-measure.cpp pmetrics-common.o
	g++ $(INCLUDE) $(CFLAGS) pmetrics-measure.cpp -c
	g++ $(INCLUDE) pmetrics-measure.o pmetrics-common.o -o pmetrics-measure  $(LIBS)

pmetrics-common.o: pmetrics-common.cpp pmetrics-common.h
	g++ $(INCLUDE) $(CFLAGS) pmetrics-common.cpp -c  $(LIBS)

clobber:
	rm -f *~ pmetrics-logger pmetrics-measure *.o

clean:
	rm -f pmetrics-measure pmetrics-logger *.o
