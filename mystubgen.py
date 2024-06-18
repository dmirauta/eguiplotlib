"""
https://stackoverflow.com/questions/49409249/python-generate-function-stubs-from-c-module
"""

import inspect
import sys

inspected_module = __import__(sys.argv[1])

with open(f"{inspected_module.__name__}.pyi", "w") as f:
    f.write(f"'''{inspected_module.__doc__}'''\n")
    for name, obj in inspect.getmembers(inspected_module):
        if inspect.isclass(obj):
            f.write("\n")
            f.write(f"class {name}:\n")

            for func_name, func in inspect.getmembers(obj):
                if not func_name.startswith("__"):
                    try:
                        f.write(f"    def {func_name} {inspect.signature(func)}:\n")
                    except:
                        f.write(f"    def {func_name} (self, *args, **kwargs):\n")
                    f.write(f"      '''{func.__doc__}'''")
                    f.write("\n    ...\n")
