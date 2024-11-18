import os
import sys
import subprocess
import tempfile
import random
import traceback
import json
import time
import shutil

pkgs = os.path.join(tempfile.gettempdir(), 'simtest-pkgs')
os.makedirs(pkgs, exist_ok=True)
sys.path.append(pkgs)

try:
  import matplotlib.pyplot
except:
  subprocess.run([
    sys.executable, '-m', 'pip', 'install', f'--target={pkgs}', 'matplotlib'
  ])
  import matplotlib.pyplot

try:
  import numpy
except:
  subprocess.run([
    sys.executable, '-m', 'pip', 'install', f'--target={pkgs}', 'numpy'
  ])
  import numpy

sys.path.append('.')
import simtest

def main():
  subprocess.run([
    'cargo', 'build', '--release'
  ], check=True)

  max_entities = 250_000
  try:
    max_entities = int(os.environ['MAX_ENTITIES'])
  except:
    pass

  print(f'MAX_ENTITIES = {max_entities}')

  num_steps_xy_data = dict()

  for num_steps in [50_000, 100_000, 250_000]:
    print(f'num_steps = {num_steps}')
    num_entities = 1024
    num_entities_to_sim_duration_d = dict()
    for sim_num in range(0, 999):
      try:
        num_entities *= 2

        if num_entities > max_entities:
          break

        begin_s = time.time()

        print(f'= = = {num_entities} entities, {num_steps} steps = = =')

        simtest.run_one_test(num_entities, num_steps)

        end_s = time.time()
        duration_s = end_s - begin_s
        num_entities_to_sim_duration_d[num_entities] = duration_s
      except:
        traceback.print_exc()
        break

    print(json.dumps(num_entities_to_sim_duration_d, indent=2))

    #data = [[1,1],[4,3],[8,3],[11,4],[10,7],[15,11],[16,12]]
    graph_points = []
    for entity_count in sorted(num_entities_to_sim_duration_d):
      graph_points.append([
        entity_count, num_entities_to_sim_duration_d[entity_count]
      ])

    x, y = zip(*graph_points)

    num_steps_xy_data[num_steps] = (x,y)


  fig, ax = matplotlib.pyplot.subplots()

  for num_steps, xy_data in num_steps_xy_data.items():
    x, y = xy_data
    ax.plot(x, y, label=f'{num_steps} steps')

  ax.set_xlabel('Entities Processed')
  ax.set_ylabel('Seconds')
  ax.set_title('Simulation run time / entities processed')
  ax.legend()

  ax.ticklabel_format(useOffset=False, style='plain') # turn off scientific notation on axes
  ax.yaxis.set_major_formatter(matplotlib.ticker.StrMethodFormatter('{x:,.0f}'))
  ax.xaxis.set_major_formatter(matplotlib.ticker.StrMethodFormatter('{x:,.0f}'))

  matplotlib.pyplot.show()




if __name__ == '__main__':
  main()
