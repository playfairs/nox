arguments = ARGV

puts "received #{arguments.length} arguments"
arguments.each_with_index do |argument, index|
  puts "[#{index + 1}] #{argument}"
end
