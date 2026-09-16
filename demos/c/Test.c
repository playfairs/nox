#include <stdio.h>

int main(int argc, char **argv) {
    printf("Hello from C (%d args)\n", argc - 1);
    return argc > 1 && argv[1][0] == 'f' ? 42 : 0;
}
