scores = [82, 91, 76, 88]
average = scores.sum.to_f / scores.length

puts "scores: #{scores.join(' ')}"
puts "minimum: #{scores.min}"
puts "maximum: #{scores.max}"
puts "average: #{format('%.2f', average)}"
