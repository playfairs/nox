let scores = [| 82; 91; 76; 88 |]
let total = Array.sum scores
let average = float total / float scores.Length

printfn "scores: %A" scores
printfn "minimum: %d" (Array.min scores)
printfn "maximum: %d" (Array.max scores)
printfn "average: %.1f" average
