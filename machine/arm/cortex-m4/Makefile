# This should generate .o files for crt0.S and entry.c
.PHONY: all clean
.DEFAULT_GOAL := all

all: crt0.S entry.c
	$(CC) $(CFLAGS) -c crt0.S -o crt0.o
	$(CC) $(CFLAGS) -c entry.c -o entry.o

clean:
	rm -f *.o