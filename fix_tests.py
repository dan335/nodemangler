"""Fix specific failing tests in noise/patterns/pbr files."""
import re

BASE = r"D:\rust\nodemangler\crates\mangler\src\operations"

def fix_file(path, old_run_test, new_run_test, old_settings=None, new_settings=None):
    with open(path, 'r', encoding='utf-8') as f:
        content = f.read()
    if old_run_test in content:
        content = content.replace(old_run_test, new_run_test)
    if old_settings and old_settings in content:
        content = content.replace(old_settings, new_settings)
    with open(path, 'w', encoding='utf-8') as f:
        f.write(content)
    print(f"Fixed: {path}")

import os

# Fix worley_distance: has 6 inputs, needs NoiseWorleyDistanceFunction at index 3
wd_path = os.path.join(BASE, "images", "noise", "worley_distance.rs")
fix_file(
    wd_path,
    # old settings assertion: left=5, right=6 means we put 5 in test but file has 6
    'assert_eq!(OpImageNoiseWorleyDistance::create_inputs().len(), 5);',
    'assert_eq!(OpImageNoiseWorleyDistance::create_inputs().len(), 6);',
)
# Also fix the run test inputs
with open(wd_path, 'r', encoding='utf-8') as f:
    content = f.read()

old_run = '''    #[tokio::test]
    async fn test_opimagenoiseworleydistance_run() {
        let mut inputs = vec![
            Input::new("i0".to_string(), Value::Integer(4), None, None),
            Input::new("i1".to_string(), Value::Integer(4), None, None),
            Input::new("i2".to_string(), Value::Integer(4), None, None),
            Input::new("i3".to_string(), Value::Integer(4), None, None),
            Input::new("i4".to_string(), Value::Integer(4), None, None)
        ];'''

new_run = '''    #[tokio::test]
    async fn test_opimagenoiseworleydistance_run() {
        let mut inputs = vec![
            Input::new("seed".to_string(), Value::Integer(1), None, None),
            Input::new("width".to_string(), Value::Integer(16), None, None),
            Input::new("height".to_string(), Value::Integer(16), None, None),
            Input::new("distance_function".to_string(), Value::NoiseWorleyDistanceFunction(NoiseWorleyDistanceFunction::EuclideanSquared), None, None),
            Input::new("frequency".to_string(), Value::Decimal(5.0), None, None),
            Input::new("i5".to_string(), Value::Integer(4), None, None),
        ];'''

if old_run in content:
    content = content.replace(old_run, new_run)
    with open(wd_path, 'w', encoding='utf-8') as f:
        f.write(content)
    print(f"Fixed run test: {wd_path}")
else:
    print(f"WARNING: pattern not found in {wd_path}")
    # Find what the test looks like
    idx = content.find('test_opimagenoiseworleydistance_run')
    if idx >= 0:
        print(repr(content[idx-20:idx+300]))

# Fix worley_value: has 6 inputs too - check
wv_path = os.path.join(BASE, "images", "noise", "worley_value.rs")
with open(wv_path, 'r', encoding='utf-8') as f:
    wv_content = f.read()

# Check actual input count
input_count = len(re.findall(r'Input::new', wv_content.split('fn create_outputs')[0].split('fn create_inputs')[1] if 'fn create_inputs' in wv_content else ''))
print(f"worley_value input count from source: checking...")
# Find create_inputs function
m = re.search(r'fn create_inputs.*?(?=fn create_outputs)', wv_content, re.DOTALL)
if m:
    n = len(re.findall(r'Input::new', m.group()))
    print(f"worley_value has {n} inputs")

# Check the settings test assertion
m2 = re.search(r'create_inputs\(\)\.len\(\), (\d+)', wv_content)
if m2:
    print(f"worley_value settings test says: {m2.group(1)}")

# Fix worley_value settings count if needed
with open(wv_path, 'r', encoding='utf-8') as f:
    wv_content = f.read()

# Get actual count
m = re.search(r'fn create_inputs\(\) -> Vec<Input> \{(.*?)fn create_outputs', wv_content, re.DOTALL)
if m:
    actual_n = len(re.findall(r'Input::new', m.group(1)))
    print(f"worley_value actual inputs: {actual_n}")

# Fix settings test to use correct count
wv_content = re.sub(
    r'(assert_eq!\(OpImageNoiseWorleyValue::create_inputs\(\)\.len\(\), )\d+(\);)',
    rf'\g<1>{actual_n}\2',
    wv_content
)

# Fix run test to use proper inputs
# The run test currently uses all Integer(4) which fails for NoiseWorleyDistanceFunction inputs
# Read what inputs worley_value actually needs
m3 = re.search(r'fn create_inputs\(\) -> Vec<Input> \{(.*?)fn create_outputs', wv_content, re.DOTALL)
if m3:
    print("worley_value create_inputs:")
    for line in m3.group(1).split('\n'):
        if 'Input::new' in line or 'Value::' in line:
            print(" ", line.strip())

with open(wv_path, 'w', encoding='utf-8') as f:
    f.write(wv_content)
