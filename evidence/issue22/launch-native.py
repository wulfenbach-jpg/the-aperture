"""Launch only this checkout's native test binary, without activation.
Print an exact PID/HWND and retain its identity; no global input or cleanup.
Usage: python evidence/issue22/launch-native.py [native flags ...]
"""
import ctypes
from ctypes import wintypes
import json
import pathlib
import subprocess
import sys
import time

root = pathlib.Path(__file__).resolve().parents[2]
exe = root / "target/release/aperture.exe"
startup = subprocess.STARTUPINFO()
startup.dwFlags = subprocess.STARTF_USESHOWWINDOW
startup.wShowWindow = 4  # SW_SHOWNOACTIVATE
log = (root / "evidence/issue22/native-stdout.log").open("a", encoding="utf-8")
process = subprocess.Popen([str(exe), *sys.argv[1:]], cwd=root, startupinfo=startup, stdout=log, stderr=subprocess.STDOUT)
user32 = ctypes.WinDLL("user32", use_last_error=True)
user32.GetWindowThreadProcessId.argtypes = [wintypes.HWND, ctypes.POINTER(wintypes.DWORD)]
user32.GetWindowThreadProcessId.restype = wintypes.DWORD
user32.IsWindowVisible.argtypes = [wintypes.HWND]
user32.IsWindowVisible.restype = wintypes.BOOL
callback_type = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
found = []
@callback_type
def visit(hwnd, unused):
    pid = wintypes.DWORD()
    user32.GetWindowThreadProcessId(hwnd, ctypes.byref(pid))
    if pid.value == process.pid and user32.IsWindowVisible(hwnd):
        found.append(int(hwnd))
    return True
for _ in range(200):
    user32.EnumWindows(visit, 0)
    if found:
        break
    if process.poll() is not None:
        raise SystemExit(f"Native process exited: {process.returncode}")
    time.sleep(0.05)
if not found:
    raise SystemExit(f"No visible window for launched PID {process.pid}")
result = {"pid": process.pid, "window": found[0], "executable": str(exe), "arguments": sys.argv[1:]}
(root / "evidence/issue22/native-instance.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
print(json.dumps(result))
