import sys
import difflib
import subprocess

args = sys.argv[1:]

print 'Running ack...'
output_ack = subprocess.Popen(['ack', '--smart-case'] + args, stdout=subprocess.PIPE).communicate()[0]
print 'Running ag...'
output_ag = subprocess.Popen(['ag'] + args, stdout=subprocess.PIPE).communicate()[0]
print 'Running ru...'
output_ru = subprocess.Popen(['ru'] + args, stdout=subprocess.PIPE).communicate()[0]
print 'Sorting...'
output_ack = sorted(output_ack.splitlines())
output_ag = sorted(output_ag.splitlines())
output_ru = sorted(output_ru.splitlines())

for line in difflib.unified_diff(output_ack, output_ag):
    print line
