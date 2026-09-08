#include <iostream>

int main(int argc, char **argv) {
    std::cout << "Hello from C++ (" << argc - 1 << " args)\n";
    return argc > 1 && argv[1][0] == 'f' ? 42 : 0;
}
