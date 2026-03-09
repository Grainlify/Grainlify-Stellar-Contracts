import os
import re

def fix_file(filepath):
    with open(filepath, 'r') as f:
        content = f.read()

    def replacer(match):
        args_str = match.group(1)
        args_str = args_str.replace('\n', ' ').strip()
        args = [arg.strip() for arg in args_str.split(',') if arg.strip()]
        if len(args) == 3:
            return f"client.create_program_release_schedule({args[1]}, {args[2]}, {args[0]})"
        return match.group(0)

    content = re.sub(r'client\.create_program_release_schedule\(([^)]*?)\)', replacer, content)

    content = re.sub(r'client\.program_exists\([^)]+\)', r'client.program_exists()', content)

    content = re.sub(r'env\.as_contract\(([^,)]+)\)', r'env.as_contract(\1, || env.clone())', content)

    with open(filepath, 'w') as f:
        f.write(content)

for root, _, files in os.walk('src'):
    for file in files:
        if file.endswith('.rs'):
            fix_file(os.path.join(root, file))
