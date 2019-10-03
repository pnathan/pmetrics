INCLUDE=-I/usr/include/postgresql
LIBS=-L/usr/lib/postgresql/9.4/lib -lpq
CFLAGS=-Wall -pedantic -Wextra -O3
.PHONY: clean

all: pmetrics-logger pmetrics-measure

pmetrics-logger: pmetrics-logger.cpp pmetrics-common.o
	g++ $(INCLUDE) $(CFLAGS) pmetrics-logger.cpp -c
	g++ $(LIBS) pmetrics-logger.o pmetrics-common.o -o pmetrics-logger

pmetrics-measure: pmetrics-measure.cpp pmetrics-common.o
	g++ $(INCLUDE) $(CFLAGS) pmetrics-measure.cpp -c
	g++ $(LIBS) pmetrics-measure.o pmetrics-common.o -o pmetrics-measure

pmetrics-common.o: pmetrics-common.cpp pmetrics-common.h
	g++ $(LIBS) $(CFLAGS) pmetrics-common.cpp -c

clobber:
	rm -f *~ pmetrics-logger pmetrics-measure *.o
