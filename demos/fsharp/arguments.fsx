let arguments = System.Environment.GetCommandLineArgs() |> Array.skip 1

printfn "received %d arguments" arguments.Length
arguments |> Array.iteri (fun index argument -> printfn "[%d] %s" (index + 1) argument)
