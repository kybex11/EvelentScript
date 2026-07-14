# Simple byte / duration formatters — inspired by rferro/normat

bytes = (n) ->
  return '0 B' if not n
  units = ['B', 'KB', 'MB', 'GB', 'TB']
  i = 0
  v = Math.abs(n)
  while v >= 1024 and i < units.length - 1
    v = v / 1024
    i += 1
  rounded = Math.floor(v * 10 + 0.5) / 10
  '' + rounded + ' ' + units[i]

ms = (n) ->
  return '0 ms' if not n
  abs = Math.abs(n)
  if abs >= 86400000
    return '' + (Math.floor((n / 86400000) * 10 + 0.5) / 10) + ' d'
  if abs >= 3600000
    return '' + (Math.floor((n / 3600000) * 10 + 0.5) / 10) + ' h'
  if abs >= 60000
    return '' + (Math.floor((n / 60000) * 10 + 0.5) / 10) + ' m'
  if abs >= 1000
    return '' + (Math.floor((n / 1000) * 10 + 0.5) / 10) + ' s'
  '' + n + ' ms'

exports.bytes = bytes
exports.ms = ms
exports.formatBytes = bytes
exports.formatMs = ms
