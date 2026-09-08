#include <stdio.h>

int main(void) {
    int scores[] = {82, 91, 76, 88};
    int total = 0;
    size_t count = sizeof(scores) / sizeof(scores[0]);

    for (size_t index = 0; index < count; ++index) {
        total += scores[index];
    }

    printf("scores: %d %d %d %d\n", scores[0], scores[1], scores[2], scores[3]);
    printf("average: %.2f\n", (double)total / count);
    return 0;
}
