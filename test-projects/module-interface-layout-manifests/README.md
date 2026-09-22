# module-interface-layout-manifests

Exercises the module-interface layout manifest proposal with public ADTs declared in one `.types.op` file, consumed through multiple modules, and used transitively from `app.op` without a direct type import.

Expected output: `command error nope`
