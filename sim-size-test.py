import os
import sys
import subprocess
import tempfile
import random
import traceback
import json

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

'''.strip())

  with open(sim_t0_data, 'w') as fd:
    fd.write(','.join(['Name', 'X0', 'Y0'])+'\n')
    for i in range(0, num_entities):
      fd.write(','.join([
        f'entity{i}', random.randint(0, 600), random.randint(0, 600),
      ])+'\n')


  with open(sim_cl_kernels, 'w') as fd:
    fd.write('''

'''.strip())

  subprocess.run([
    os.path.join('target', 'release', 'apollon'),
    sim_control_toml,
      '--num-steps', f'{num_steps}',
      '--capture-step-period', '999999999',

  ])


  if not 'NO_REMOVE_SIMS' in os.environ:
    shutil.rmtree(sim_dir)

def main():
  subprocess.run([
    'cargo', 'build', '--release'
  ], check=True)

  num_steps = 9000

  num_entities = 16
  num_entities_to_sim_duration_d = dict()
  for sim_num in range(0, 999):
    try:
      num_entities *= 2
      begin_s = time.time()
      print(f'= = = {num_entities} entities, {num_steps} steps = = =')

      end_s = time.time()
      duration_s = end_s - begin_s
      num_entities_to_sim_duration_d[num_entities] = duration_s
    except:
      traceback.print_exc()
      break

  print(json.dumps(num_entities_to_sim_duration_d, indent=2))





if __name__ == '__main__':
  main()
