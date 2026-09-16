#include <iostream>

int main(int argc, char **argv) {
    std::cout << "received " << argc - 1 << " arguments\n";
    for (int index = 1; index < argc; ++index) {
        std::cout << '[' << index << "] " << argv[index] << '\n';
    }
}
