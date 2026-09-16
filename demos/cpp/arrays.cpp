#include <array>
#include <iostream>
#include <numeric>

int main() {
    std::array<int, 4> scores{82, 91, 76, 88};
    const int total = std::accumulate(scores.begin(), scores.end(), 0);

    std::cout << "scores:";
    for (int score : scores) {
        std::cout << ' ' << score;
    }
    std::cout << "\naverage: " << static_cast<double>(total) / scores.size() << '\n';
}
