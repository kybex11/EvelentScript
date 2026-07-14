# Easing equations — EvelentScript port of jimjeffers/Easie

linear = (time, begin, change, duration) ->
  change * time / duration + begin

quadIn = (time, begin, change, duration) ->
  time = time / duration
  change * time * time + begin

quadOut = (time, begin, change, duration) ->
  time = time / duration
  -change * time * (time - 2) + begin

quadInOut = (time, begin, change, duration) ->
  time = time / (duration / 2)
  if time < 1
    return change / 2 * time * time + begin
  time -= 1
  -change / 2 * (time * (time - 2) - 1) + begin

cubicIn = (time, begin, change, duration) ->
  time = time / duration
  change * time * time * time + begin

cubicOut = (time, begin, change, duration) ->
  time = time / duration - 1
  change * (time * time * time + 1) + begin

cubicInOut = (time, begin, change, duration) ->
  time = time / (duration / 2)
  if time < 1
    return change / 2 * time * time * time + begin
  time -= 2
  change / 2 * (time * time * time + 2) + begin

sineIn = (time, begin, change, duration) ->
  -change * Math.cos(time / duration * (Math.PI / 2)) + change + begin

sineOut = (time, begin, change, duration) ->
  change * Math.sin(time / duration * (Math.PI / 2)) + begin

sineInOut = (time, begin, change, duration) ->
  -change / 2 * (Math.cos(Math.PI * time / duration) - 1) + begin

expoIn = (time, begin, change, duration) ->
  return begin if time is 0
  change * Math.pow(2, 10 * (time / duration - 1)) + begin

expoOut = (time, begin, change, duration) ->
  return begin + change if time is duration
  change * (-Math.pow(2, -10 * time / duration) + 1) + begin

circIn = (time, begin, change, duration) ->
  time = time / duration
  -change * (Math.sqrt(1 - time * time) - 1) + begin

circOut = (time, begin, change, duration) ->
  time = time / duration - 1
  change * Math.sqrt(1 - time * time) + begin

exports.linear = linear
exports.quadIn = quadIn
exports.quadOut = quadOut
exports.quadInOut = quadInOut
exports.cubicIn = cubicIn
exports.cubicOut = cubicOut
exports.cubicInOut = cubicInOut
exports.sineIn = sineIn
exports.sineOut = sineOut
exports.sineInOut = sineInOut
exports.expoIn = expoIn
exports.expoOut = expoOut
exports.circIn = circIn
exports.circOut = circOut
