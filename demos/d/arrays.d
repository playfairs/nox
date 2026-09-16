import std.stdio;

void main() {
    int[] scores = [82, 91, 76, 88];
    int total = 0;

    foreach (score; scores) {
        total += score;
    }

    writeln("scores: ", scores);
    writeln("average: ", cast(double) total / scores.length);
}
