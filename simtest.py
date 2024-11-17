import os
import sys
import subprocess
import tempfile
import random
import traceback
import json
import time
import shutil


def run_one_test(num_entities, num_steps):
  sim_dir = os.path.join(tempfile.gettempdir(), f'sim_{num_entities}_entities_{num_steps}_steps')
  os.makedirs(sim_dir, exist_ok=True)

  try:
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
  ['red_entity_speed_coef', 'float', 1.75 ],
  ['blue_entity_speed_coef', 'float', 1.0 ],
]

# This string is passed verbatim to the compiler backend.
# Most users will not need these, only those chasing extreme performance will care.
# Option effects are documented at https://registry.khronos.org/OpenCL/specs/3.0-unified/html/OpenCL_API.html#compiler-options
cl_program_compiler_options = ""

source = """
kernel void compute_position (
    global float* X0,
    global float* Y0,
    global char* entity_x_direction, // 0 == positive 1 == negative
    global char* entity_y_direction,
    float blue_entity_speed_coef,
    float red_entity_speed_coef
)
{
    const size_t i = get_global_id(0);
    if (i == 0) {
      if (entity_x_direction[i] == 0) {
        X0[i] = X0[i] + (blue_entity_speed_coef);
      }
      else {
        X0[i] = X0[i] - (blue_entity_speed_coef);
      }

      if (entity_y_direction[i] == 0) {
        Y0[i] = Y0[i] + (blue_entity_speed_coef);
      }
      else {
        Y0[i] = Y0[i] - (blue_entity_speed_coef);
      }

      if (X0[i] > 500) {
        entity_x_direction[i] = 1;
      }
      if (X0[i] < 50) {
        entity_x_direction[i] = 0;
      }
      if (Y0[i] > 400) {
        entity_y_direction[i] = 1;
      }
      if (Y0[i] < 40) {
        entity_y_direction[i] = 0;
      }

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

      // We also move away from our neighbor (i-1)
      float x_dist_to_i1 = X0[i] - X0[i-1];
      float y_dist_to_i1 = Y0[i] - Y0[i-1];
      X0[i] = X0[i] + (red_entity_speed_coef * (x_dist_to_i1 / 100.0) );
      Y0[i] = Y0[i] + (red_entity_speed_coef * (y_dist_to_i1 / 100.0) );


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
  except:
    if not 'NO_REMOVE_SIMS' in os.environ:
      shutil.rmtree(sim_dir)

    raise

  if not 'NO_REMOVE_SIMS' in os.environ:
    shutil.rmtree(sim_dir)

