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


def run_one_test(num_entities, num_steps):
  sim_dir = os.path.join(tempfile.gettempdir(), f'sim_{num_entities}_entities_{num_steps}_steps')
  os.makedirs(sim_dir, exist_ok=True)

  sim_control_toml = os.path.join(sim_dir, 'simcontrol.toml')
  sim_t0_data =      os.path.join(sim_dir, 'in_data.csv')
  sim_cl_kernels =   os.path.join(sim_dir, 'cl-kernels.toml')

  with open(sim_control_toml, 'w') as fd:
    fd.write(f'''
[simulation]
input_data_file_path = "{sim_t0_data}"
cl_kernels_file_path = "{sim_cl_kernels}"

gis_x_attr_name = "X0"
gis_y_attr_name = "Y0"

gis_name_attr = "Name"

[data_constants]

'''.strip()+'\n')

  with open(sim_t0_data, 'w') as fd:
    fd.write(','.join(['Name', 'X0', 'Y0'])+'\n')
    for i in range(0, num_entities):
      fd.write(','.join([
        f'entity{i}', f'{random.randint(0, 600)}', f'{random.randint(0, 600)}',
      ])+'\n')


  with open(sim_cl_kernels, 'w') as fd:
    fd.write('''
[[kernel]]
# The kernel `name` MUST match a kernel defined in the `source` field.
name = "compute_position"

# [colmap] is a dictionary with keys containing
# source arg names and values containing Data column names.
# we get type data by querying parsed source directly and convert from the original to the processing target types.
colmap.x0 = 'X0'
colmap.y0 = 'Y0'

# Constants is a list of keys -> value data.
#   1st element is Name of the variable; this is only used for diagnostic & reporting reasons
#   The 2nd element of the value data is a string denoting type
#   and the 3rd element is a numeric value which will be assigned to that type.
# Constant variables are NOT pointers, and get passed in as their type in the order specified here.
# for that reason, order in this list MUST MATCH ordering in your kernel's `source` function.
data_constants = [
  ['red_entity_speed_coef', 'float', 1.5 ],
  ['blue_entity_speed_coef', 'float', 2.0 ],
#  ['another_var',   'int64', 999 ],
]

# This string is passed verbatim to the compiler backend.
# Most users will not need these, only those chasing extreme performance will care.
# Option effects are documented at https://registry.khronos.org/OpenCL/specs/3.0-unified/html/OpenCL_API.html#compiler-options
cl_program_compiler_options = ""

source = """
kernel void compute_position (
    global float* X0,
    global float* Y0,
    float blue_entity_speed_coef,
    float red_entity_speed_coef
)
{
    const size_t i = get_global_id(0);
    if (i == 0) {
      X0[i] = X0[i] + (blue_entity_speed_coef);
      Y0[i] = Y0[i] + (blue_entity_speed_coef);
    }
    else {

      // TODO add weight & figure out a momentum calculation

      float x_dist_to_i0 = X0[i] - X0[0];
      float y_dist_to_i0 = Y0[i] - Y0[0];
      float dist = fabs(x_dist_to_i0) + fabs(y_dist_to_i0);
      if (dist > 25.0) {
        // Move 10% faster
        X0[i] = X0[i] + (red_entity_speed_coef * (-x_dist_to_i0 / 90.0) );
        Y0[i] = Y0[i] + (red_entity_speed_coef * (-y_dist_to_i0 / 90.0) );
      }
      else {
        // Move slower
        X0[i] = X0[i] + (red_entity_speed_coef * (-x_dist_to_i0 / 100.0) );
        Y0[i] = Y0[i] + (red_entity_speed_coef * (-y_dist_to_i0 / 100.0) );
      }
    }
}
"""



'''.strip())

  subprocess.run([
    os.path.join('target', 'release', 'apollon'),
    sim_control_toml,
      '--num-steps', f'{num_steps}',
      '--capture-step-period', '999999999',
  ], check=True)


  if not 'NO_REMOVE_SIMS' in os.environ:
    shutil.rmtree(sim_dir)

def main():
  subprocess.run([
    'cargo', 'build', '--release'
  ], check=True)

  num_steps = 9000

  num_entities = 128
  num_entities_to_sim_duration_d = dict()
  for sim_num in range(0, 999):
    try:
      num_entities *= 2
      begin_s = time.time()

      print(f'= = = {num_entities} entities, {num_steps} steps = = =')

      run_one_test(num_entities, num_steps)

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

  fig, ax = matplotlib.pyplot.subplots()
  ax.plot(x, y)

  ax.set_xlabel('Entities Processed')
  ax.set_ylabel('Seconds')
  ax.set_title('Simulation run time / entities processed')

  ax.ticklabel_format(useOffset=False, style='plain') # turn off scientific notation on axes
  ax.yaxis.set_major_formatter(matplotlib.ticker.StrMethodFormatter('{x:,.0f}'))
  ax.xaxis.set_major_formatter(matplotlib.ticker.StrMethodFormatter('{x:,.0f}'))

  matplotlib.pyplot.show()




if __name__ == '__main__':
  main()
