"""Run the opt-in native frame report twice on this checkout's binary.
Run B holds a real blocking local-model request (Decider-2B endpoint, which is
NOT running here) on the background worker while frames are sampled. No remote
network service, no GPU/model process: the request stalls on an absent loopback
endpoint and times out. Run A has no worker activity.
"""
import ctypes, json, pathlib, subprocess, sys, time
from ctypes import wintypes

root = pathlib.Path(__file__).resolve().parents[2]
exe = root / "target/release/aperture.exe"
user32 = ctypes.WinDLL("user32", use_last_error=True)


def window_for(pid: int) -> int:
    found = []
    cb = ctypes.WINFUNCTYPE(wintypes.BOOL, wintypes.HWND, wintypes.LPARAM)
    @cb
    def visit(h, _):
        owner = wintypes.DWORD()
        user32.GetWindowThreadProcessId(h, ctypes.byref(owner))
        if owner.value == pid and user32.IsWindowVisible(h):
            found.append(int(h))
        return True
    for _ in range(300):
        user32.EnumWindows(visit, 0)
        if found:
            return found[0]
        time.sleep(0.05)
    raise SystemExit(f"no window for {pid}")


def launch(report: pathlib.Path, note: str | None):
    args = [str(exe), "--seats", "4", "--profile", "bounded", "--size", "1280x720",
            "--performance-report", str(report)]
    if note:
        args += ["--performance-note", note]
    si = subprocess.STARTUPINFO()
    si.dwFlags = subprocess.STARTF_USESHOWWINDOW
    si.wShowWindow = 4
    log = (root / "evidence/issue22/native-stdout.log").open("a", encoding="utf-8")
    proc = subprocess.Popen(args, cwd=root, startupinfo=si, stdout=log, stderr=subprocess.STDOUT)
    return proc, window_for(proc.pid)


def uia(hwnd: int, script: str):
    out = root / "evidence/issue22/captures/uia-last.json"
    result = subprocess.run(["powershell.exe", "-NoProfile", "-ExecutionPolicy", "Bypass",
                             "-File", str(root / "evidence/issue22/uia-step.ps1"),
                             "-Window", str(hwnd), "-Script", script, "-Out", str(out)],
                            capture_output=True, text=True)
    if result.returncode != 0:
        raise SystemExit(f"UIA step failed: {result.stdout} {result.stderr}")
    return json.loads(out.read_text(encoding="utf-8-sig"))


def wait_report(report: pathlib.Path, limit=40.0):
    started = time.time()
    while time.time() - started < limit:
        if report.exists() and report.stat().st_size > 200:
            time.sleep(0.4)
            return json.loads(report.read_text(encoding="utf-8"))
        time.sleep(0.2)
    raise SystemExit(f"report never appeared: {report}")


def stop(proc):
    proc.terminate()
    try:
        proc.wait(timeout=5)
    except subprocess.TimeoutExpired:
        proc.kill()


results = {}


def wait_control(hwnd, name, limit=12.0):
    """Poll the accessibility tree until `name` is present (startup is async)."""
    probe = (
        "$e=$root.FindFirst([System.Windows.Automation.TreeScope]::Descendants,"
        "(New-Object System.Windows.Automation.PropertyCondition("
        f"[System.Windows.Automation.AutomationElement]::NameProperty,'{name}')));"
        "if($e){'present'}"
    )
    started = time.time()
    while time.time() - started < limit:
        try:
            if uia(hwnd, probe):
                return True
        except SystemExit:
            pass
        time.sleep(0.5)
    return False


# A: identical instrumentation, no background work.
proc, hwnd = launch(root / "evidence/issue22/perf-baseline.json", None)
try:
    results["A_baseline"] = wait_report(root / "evidence/issue22/perf-baseline.json")
finally:
    stop(proc)

# B: toggling the Decider starts the real health request to the absent loopback
# endpoint, so the background worker is blocked while frames keep being sampled.
proc, hwnd = launch(
    root / "evidence/issue22/perf-delayed-worker.json",
    "absent local-model endpoint (loopback 127.0.0.1:8765 not running) held the background worker; controlled condition, not real model latency",
)
try:
    results["B_decider_control_present"] = wait_control(hwnd, "Use local Decider-2B")
    step = uia(hwnd,
        "$d=$root.FindFirst([System.Windows.Automation.TreeScope]::Descendants,"
        "(New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::NameProperty,'Use local Decider-2B')));"
        "if(!$d){throw 'missing decider toggle'};"
        "$d.GetCurrentPattern([System.Windows.Automation.TogglePattern]::Pattern).Toggle();"
        "Start-Sleep -Milliseconds 500;"
        "@(([System.Windows.Automation.AutomationElement]::FromHandle([IntPtr]$hwnd)).FindAll([System.Windows.Automation.TreeScope]::Descendants,"
        "[System.Windows.Automation.Condition]::TrueCondition) | ForEach-Object {$_.Current.Name} | Where-Object {$_ -like 'Working*' -or $_ -like '*decider*' -or $_ -like 'WAITING*'})"
    )
    results["B_worker_step"] = step
    results["B_delayed_worker"] = wait_report(root / "evidence/issue22/perf-delayed-worker.json")
finally:
    stop(proc)
(root / "evidence/issue22/perf-comparison.json").write_text(
    json.dumps(results, indent=2), encoding="utf-8")
for name in ("A_baseline", "B_delayed_worker"):
    report = results[name]
    print(name, "samples", report["samples"],
          "cb_p95_us", report["callback_duration_us"]["p95"],
          "cb_max_us", report["callback_duration_us"]["max"],
          "interval_p95_us", report["inter_callback_interval_us"]["p95"],
          "input_frames", report["input_frame_render_us"]["samples"],
          "synthetic", report["synthetic_delays"])
print("B step observation:", results["B_worker_step"])
