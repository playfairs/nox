Todo List
===============================

Additions
---------

* A "Run" feature
	* E.g FSharp with FSX, you can express `nox run Test.fsx` and obviously if fsx,
	  is defaulted to dotnet fsi scripting then it will know to run `dotnet fsi Test.fsx`

	* E.g C++, say gcc, it doesn't have a run built in, so you replicate it by compiling,
	  then executing the binary, a good handler solution for something like that as a bonus
	  is outputting the binary to a temporary location, and then gets marked for deletion

* Variables/Let-bindings

* Semi-turing completeness
    * It is helpful to be able to have a system which will be able to do some sort of math,
	  it can can be used in conjuction with possibly variables/let-bindings feature

	* E.g you have a `x` and you want to set it globally to use `3` but then you have a area
	  which wants `x/2` in this case `x/2 -> 1.5`, this is helpful

* Package manager
	* Mostly for C/C++, but is a great addition due to the language being pretty much
	  useless/inconvenient without one
