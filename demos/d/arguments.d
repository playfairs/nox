import std.stdio;

void main(string[] arguments) {
    writeln("received ", arguments.length - 1, " arguments");
    foreach (index, argument; arguments[1 .. $]) {
        writeln("[", index + 1, "] ", argument);
    }
}
