#include <stdio.h>

int main(int argc, char **argv) {
    printf("received %d arguments\n", argc - 1);
    for (int index = 1; index < argc; ++index) {
        printf("[%d] %s\n", index, argv[index]);
    }
    return 0;
}
